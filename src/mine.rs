
use std::{sync::Arc, sync::RwLock, time::Instant};

use coal_guilds_api;
use colored::*;
use drillx::{
    equix::{self},
    Hash, Solution,
};
use coal_api::{consts::*, state::{Config, config_pda, proof_pda}};
use ore_api::state::proof_pda as ore_proof_pda;
use ore_boost_api::state::reservation_pda as ore_reservation_pda;
use steel::AccountDeserialize;
use rand::Rng;
use solana_program::{pubkey::Pubkey, instruction::Instruction, system_program};
use solana_rpc_client::spinner;
use solana_sdk::signer::Signer;


use crate::{
    args::MineArgs,
    send_and_confirm::ComputeBudget,
    guild::utils::{deserialize_member, deserialize_guild, deserialize_config as deserialize_guild_config},
    utils::{amount_u64_to_string, get_clock, get_config, deserialize_config, get_updated_ore_proof_with_authority, deserialize_proof, deserialize_reservation},
    Miner,
};

impl Miner {
    pub async fn mine(&self, args: MineArgs) {
        // Open account, if needed.
        let mint = MINT_ADDRESS;
        let signer = self.signer();

        self.open().await;
        
        // Check num threads
        self.check_num_cores(args.cores);

        println!("{} {}", "INFO".bold().green(), "Started mining");

        // Start mining loop
        let mut last_hash_at = 0;
        let mut last_balance = 0;

        let mut guild: Option<coal_guilds_api::state::Guild> = None;
        let mut guild_address: Option<Pubkey> = None;


        let config_address = config_pda(mint).0;
        let proof_address = proof_pda(mint, signer.pubkey()).0;
        let ore_proof_address = ore_proof_pda(proof_address).0;
        let ore_reservation_address = ore_reservation_pda(ore_proof_address).0;
        let guild_config_address = coal_guilds_api::state::config_pda().0;
        let guild_member_address = coal_guilds_api::state::member_pda(signer.pubkey()).0;
    
        loop {    
            let ore_proof = get_updated_ore_proof_with_authority(&self.rpc_client, proof_address, last_hash_at).await;

            let mut addresses: Vec<Pubkey> = vec![config_address, proof_address, ore_reservation_address, guild_config_address, guild_member_address];
            if guild_address.is_some() {
                addresses.push(guild_address.unwrap());
            }
            let accounts = self.rpc_client.get_multiple_accounts(&addresses).await.unwrap();
            
            let config = deserialize_config(&accounts[0].as_ref().unwrap().data);
            let proof = deserialize_proof(&accounts[1].as_ref().unwrap().data);
            let reservation = deserialize_reservation(&accounts[2].as_ref().unwrap().data);

            let mut member: Option<coal_guilds_api::state::Member> = None;
            let mut guild_config: Option<coal_guilds_api::state::Config> = None;
            
            if accounts.len() > 1 {
                if accounts[2].as_ref().is_some() {
                    guild_config = Some(deserialize_guild_config(&accounts[2].as_ref().unwrap().data));
                }
            
                if accounts[3].as_ref().is_some() {
                    member = Some(deserialize_member(&accounts[3].as_ref().unwrap().data));
                }

                if accounts.len() > 4 && accounts[4].as_ref().is_some() {
                    guild = Some(deserialize_guild(&accounts[4].as_ref().unwrap().data));
                }
            }

            if member.is_some() && member.unwrap().guild.ne(&system_program::id()) && guild_address.is_none() {
                let guild_data = self.rpc_client.get_account_data(&member.unwrap().guild).await.unwrap();
                guild = Some(deserialize_guild(&guild_data));
                guild_address = Some(member.unwrap().guild);
            }

            let total_balance = config.total_balance;
            let min_difficulty = config.min_difficulty;
            
            println!(
                "\n\nStake: {} COAL\n {} Stake Multiplier: {:12}xx\n LP Stake Multiplier: {:12}x",
                amount_u64_to_string(proof.balance),
                if last_hash_at.gt(&0) {
                    format!(
                        "  Change: {} COAL\n",
                        amount_u64_to_string(proof.balance.saturating_sub(last_balance)),
                    )
                } else {
                    "".to_string()
                },
                calculate_multiplier(proof.balance, total_balance),
                match guild_config {
                    Some(guild_config) => {
                        match guild {
                            Some(guild) => calculate_stake_multiplier(guild.total_stake, guild_config.total_stake, guild_config.total_multiplier),
                            None => if member.is_some() { 
                                calculate_stake_multiplier(member.unwrap().total_stake, guild_config.total_stake, guild_config.total_multiplier)
                            } else { 
                                0.0
                            },
                        }
                    },
                    None => 0.0,
                }
            );

            // Calculate cutoff time
            let cutoff_time = self.get_cutoff(ore_proof.last_hash_at, ONE_MINUTE, args.buffer_time).await;

            // Run drillx
            let solution = Self::find_hash_par(ore_proof.challenge, cutoff_time, args.cores, min_difficulty as u32)
                .await;


            // Build instruction set
            let mut compute_budget = 500_000;
            let mut ixs: Vec<Instruction> = vec![
                ore_api::sdk::auth(ore_api::state::proof_pda(proof_address).0)
            ];

            // Reset if needed
            let config = get_config(&self.rpc_client, mint).await;
            
            if self.should_reset(config).await {
                compute_budget += 100_000;
                ixs.push(coal_api::sdk::reset(mint, signer.pubkey()));
            }

            // Build mine ix
            let ore_boost_address = if reservation.boost == Pubkey::default() {
                None
            } else {
                Some(reservation.boost)
            };

            ixs.push(coal_api::sdk::mine(
                mint,
                signer.pubkey(),
                signer.pubkey(),
                self.find_bus().await,
                self.find_ore_bus().await,
                solution,
                match member {
                    Some(_) => Some(guild_member_address),
                    None => None
                },
                match guild {
                    Some(_) => guild_address,
                    None => None
                },
                ore_boost_address,
            ));

            // Submit transactions
            self.send_and_confirm(&ixs, ComputeBudget::Fixed(compute_budget), false).await.ok();
        }

    }

    pub async fn find_hash_par(
        challenge: [u8; 32],
        cutoff_time: u64,
        cores: u64,
        min_difficulty: u32,
    ) -> Solution {
        // Dispatch job to each thread
        let progress_bar = Arc::new(spinner::new_progress_bar());
        let global_best_difficulty = Arc::new(RwLock::new(0u32));
        progress_bar.set_message("Mining...");
        let core_ids = core_affinity::get_core_ids().unwrap();
        let handles: Vec<_> = core_ids
            .into_iter()
            .map(|i| {
                let global_best_difficulty = Arc::clone(&global_best_difficulty);
                std::thread::spawn({
                    let progress_bar = progress_bar.clone();
                    let mut memory = equix::SolverMemory::new();
                    move || {
                        // Return if core should not be used
                        if (i.id as u64).ge(&cores) {
                            return (0, 0, Hash::default());
                        }

                        // Pin to core
                        let _ = core_affinity::set_for_current(i);

                        // Start hashing
                        let timer = Instant::now();
                        let mut nonce = u64::MAX.saturating_div(cores).saturating_mul(i.id as u64);
                        let mut best_nonce = nonce;
                        let mut best_difficulty = 0;
                        let mut best_hash = Hash::default();
                        loop {
                            // Get hashes
                            let hxs = drillx::hashes_with_memory(
                                &mut memory,
                                &challenge,
                                &nonce.to_le_bytes(),
                            );
                             // Look for best difficulty score in all hashes
                             for hx in hxs {
                                let difficulty = hx.difficulty();
                                if difficulty.gt(&best_difficulty) {
                                    best_nonce = nonce;
                                    best_difficulty = difficulty;
                                    best_hash = hx;
                                    if best_difficulty.gt(&*global_best_difficulty.read().unwrap())
                                    {
                                        *global_best_difficulty.write().unwrap() = best_difficulty;
                                    }
                                }
                            }
            

                            // Exit if time has elapsed
                            if nonce % 100 == 0 {
                                let global_best_difficulty =
                                    *global_best_difficulty.read().unwrap();
                                if timer.elapsed().as_secs().ge(&cutoff_time) {
                                    if i.id == 0 {
                                        progress_bar.set_message(format!(
                                            "Mining... (difficulty {})",
                                            global_best_difficulty,
                                        ));
                                    }
                                    if global_best_difficulty.ge(&min_difficulty) {
                                        // Mine until min difficulty has been met
                                        break;
                                    }
                                } else if i.id == 0 {
                                    progress_bar.set_message(format!(
                                        "Mining... (difficulty {}, time {})",
                                        global_best_difficulty,
                                        format_duration(
                                            cutoff_time.saturating_sub(timer.elapsed().as_secs())
                                                as u32
                                        ),
                                    ));
                                }
                            }

                            // Increment nonce
                            nonce += 1;
                        }

                        // Return the best nonce
                        (best_nonce, best_difficulty, best_hash)
                    }
                })
            })
            .collect();

        // Join handles and return best nonce
        let mut best_nonce = 0;
        let mut best_difficulty = 0;
        let mut best_hash = Hash::default();
        for h in handles {
            if let Ok((nonce, difficulty, hash)) = h.join() {
                if difficulty > best_difficulty {
                    best_difficulty = difficulty;
                    best_nonce = nonce;
                    best_hash = hash;
                }
            }
        }

        // Update log
        progress_bar.finish_with_message(format!(
            "Best hash: {} (difficulty {})",
            bs58::encode(best_hash.h).into_string(),
            best_difficulty
        ));

        Solution::new(best_hash.d, best_nonce.to_le_bytes())
    }

    pub fn check_num_cores(&self, cores: u64) {
        let num_cores = num_cpus::get() as u64;
        if cores.gt(&num_cores) {
            println!(
                "{} Cannot exceeds available cores ({})",
                "WARNING".bold().yellow(),
                num_cores
            );
        }
    }

    pub async fn should_reset(&self, config: Config) -> bool {
        let clock = get_clock(&self.rpc_client).await;
        config.last_reset_at
            .saturating_add(EPOCH_DURATION)
            .saturating_sub(5) // Buffer
            .le(&clock.unix_timestamp)
    }

    pub async fn get_cutoff(&self, last_hash_at: i64, duration: i64, buffer_time: u64) -> u64 {
        let clock = get_clock(&self.rpc_client).await;
        last_hash_at
            .saturating_add(duration)
            .saturating_sub(buffer_time as i64)
            .saturating_sub(clock.unix_timestamp)
            .max(0) as u64
    }

    pub async fn find_ore_bus(&self) -> Pubkey {
        // Fetch the bus with the largest balance
        let bus_addresses = ore_api::consts::BUS_ADDRESSES;
        if let Ok(accounts) = self.rpc_client.get_multiple_accounts(&bus_addresses).await {
            let mut top_bus_balance: u64 = 0;
            let mut top_bus = bus_addresses[0];
            for account in accounts {
                if let Some(account) = account {
                    if let Ok(bus) = ore_api::state::Bus::try_from_bytes(&account.data) {
                        if bus.rewards.gt(&top_bus_balance) {
                            top_bus_balance = bus.rewards;
                            top_bus = bus_addresses[bus.id as usize];
                        }
                    }

                }
            }
            return top_bus;
        }

        // Otherwise return a random bus
        let i = rand::thread_rng().gen_range(0..BUS_COUNT);
        bus_addresses[i]
    }

    pub async fn find_bus(&self) -> Pubkey {
        // Fetch the bus with the largest balance
        let bus_addresses = coal_api::consts::BUS_ADDRESSES;
        if let Ok(accounts) = self.rpc_client.get_multiple_accounts(&bus_addresses).await {
            let mut top_bus_balance: u64 = 0;
            let mut top_bus = bus_addresses[0];
            for account in accounts {
                if let Some(account) = account {
                    if let Ok(bus) = coal_api::state::Bus::try_from_bytes(&account.data) {
                        if bus.rewards.gt(&top_bus_balance) {
                            top_bus_balance = bus.rewards;
                            top_bus = bus_addresses[bus.id as usize];
                        }
                    }

                }
            }
            return top_bus;
        }

        // Otherwise return a random bus
        let i = rand::thread_rng().gen_range(0..BUS_COUNT);
        bus_addresses[i]
    }
}

// TODO
fn calculate_multiplier(balance: u64, total_balance: u64) -> f64 {
    if total_balance == 0 {
        return 0.0;
    }
    16.0 * (balance as f64) / (total_balance as f64)
}

fn calculate_stake_multiplier(stake: u64, total_stake: u64, total_multiplier: u64) -> f64 {
    if total_stake == 0 {
        return 0.0;
    }
    (total_multiplier as f64) * (stake as f64) / (total_stake as f64)
}

fn format_duration(seconds: u32) -> String {
    let minutes = seconds / 60;
    let remaining_seconds = seconds % 60;
    format!("{:02}:{:02}", minutes, remaining_seconds)
}