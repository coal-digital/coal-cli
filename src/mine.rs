use std::{sync::Arc, sync::RwLock, time::Instant};
use colored::*;
use drillx_2::{
    equix::{self},
    Hash, Solution,
};
use coal_api::{
    consts::{BUS_ADDRESSES, BUS_COUNT, EPOCH_DURATION},
    state::{Bus, Config, Proof},
};
use ore_api::consts::BUS_ADDRESSES as ORE_BUS_ADDRESSES;
use smelter_api::consts::BUS_ADDRESSES as SMELTER_BUS_ADDRESSES;
use coal_utils::AccountDeserialize;
use rand::Rng;
use solana_program::pubkey::Pubkey;
use solana_rpc_client::spinner;
use solana_sdk::signer::Signer;
use tokio;

use crate::{
    args::MineArgs,
    send_and_confirm::ComputeBudget,
    utils::{
        Resource,
        amount_u64_to_string,
        get_clock, get_config,
        get_updated_proof_with_authority, 
        proof_pubkey,
    },
    Miner,
};

impl Miner {
    pub async fn mine(&self, args: MineArgs) {
        // Open account, if needed.
        let merged = args.merged.clone();

        match merged.as_str() {
            "ore" => {
                println!("{} {}", "INFO".bold().green(), "Started merged mining...");
            }
            "none" => {
                println!("{} {}", "INFO".bold().green(), "Started coal mining...");
            }
            _ => {
                println!("{} {} \"{}\" {}", "ERROR".bold().red(), "Argument value", merged, "not recognized");
                return;
            }
        }

        let signer = self.signer();
        let result = self.open(args.merged).await;
        if result.is_err() {
            println!("{} {}", "ERROR".bold().red(), result.err().unwrap());
            return;
        }

        // Check num threads
        self.check_num_cores(args.cores);

        // Start mining loop
        let mut last_coal_hash_at = 0;
        let mut last_coal_balance = 0;
        let mut last_ore_hash_at = 0;
        let mut last_ore_balance = 0;
        loop {
            // Fetch coal_proof
            let (coal_config, ore_config) = tokio::join!(
                get_config(&self.rpc_client, Resource::Coal),
                get_config(&self.rpc_client, Resource::Ore)
            );
            let coal_proof = get_updated_proof_with_authority(&self.rpc_client, signer.pubkey(), last_coal_hash_at, Resource::Coal).await;
            let ore_proof = match merged.as_str() {
                "ore" => get_updated_proof_with_authority(&self.rpc_client, signer.pubkey(), last_ore_hash_at, Resource::Ore).await,
                _ => coal_proof,
            };

            println!(
                "\n\nStake: {} COAL\n{}  Multiplier: {:12}x",
                amount_u64_to_string(coal_proof.balance),
                if last_coal_hash_at.gt(&0) {
                    format!(
                        "  Change: {} COAL\n",
                        amount_u64_to_string(coal_proof.balance.saturating_sub(last_coal_balance))
                    )
                } else {
                    "".to_string()
                },
                calculate_multiplier(coal_proof.balance, coal_config.top_balance)
            );

            match merged.as_str() {
                "ore" => {
                    println!(
                        "Stake: {} ORE\n{}  Multiplier: {:12}x",
                        amount_u64_to_string(ore_proof.balance),
                        if last_ore_hash_at.gt(&0) {
                            format!(
                                "  Change: {} ORE\n",
                                amount_u64_to_string(ore_proof.balance.saturating_sub(last_ore_balance))
                            )
                        } else {
                            "".to_string()
                        },
                        calculate_multiplier(ore_proof.balance, ore_config.top_balance)
                    );
                }
                _ => {}
            }
            

            last_coal_hash_at = coal_proof.last_hash_at;
            last_coal_balance = coal_proof.balance;
            last_ore_hash_at = ore_proof.last_hash_at;
            last_ore_balance = ore_proof.balance;

            // Calculate cutoff time
            let cutoff_time = self.get_cutoff(coal_proof, args.buffer_time).await;

            // Run drillx_2
            let min_difficulty = match merged.as_str() { 
                "ore" => coal_config.min_difficulty.max(ore_config.min_difficulty),
                _ => coal_config.min_difficulty,
            };
            let solution = Self::find_hash_par(coal_proof, cutoff_time, args.cores, min_difficulty as u32, Resource::Coal)
                .await;


            let mut compute_budget = 500_000;
            // Build instruction set
            let mut ixs = vec![
                ore_api::instruction::auth(proof_pubkey(signer.pubkey(), Resource::Ore)),
                coal_api::instruction::auth(proof_pubkey(signer.pubkey(), Resource::Coal)),
            ];

            match merged.as_str() {
                "ore" => {
                    compute_budget += 500_000;
                    ixs.push(coal_api::instruction::mine_ore(
                        signer.pubkey(),
                        signer.pubkey(),
                        self.find_bus(Resource::Ore).await,
                        solution,
                    ));
                }
                _ => {}
            }

            // Reset if needed
            let coal_config = get_config(&self.rpc_client, Resource::Coal).await;
            if self.should_reset(coal_config).await {
                compute_budget += 100_000;
                ixs.push(coal_api::instruction::reset(signer.pubkey()));
            }

            // Build mine ix
            ixs.push(coal_api::instruction::mine(
                signer.pubkey(),
                signer.pubkey(),
                self.find_bus(Resource::Coal).await,
                solution,
            ));

            // Submit transactions
            self.send_and_confirm(&ixs, ComputeBudget::Fixed(compute_budget), false).await.ok();
        }
    }

    pub async fn find_hash_par(
        coal_proof: Proof,
        cutoff_time: u64,
        cores: u64,
        min_difficulty: u32,
        resource: Resource,
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
                    let coal_proof = coal_proof.clone();
                    let progress_bar = progress_bar.clone();
                    let mut memory = equix::SolverMemory::new();
                    let resource = resource.clone(); // Clone resource here
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
                            // Create hash
                            for hx in drillx_2::get_hashes_with_memory(&mut memory, &coal_proof.challenge, &nonce.to_le_bytes()) {
                                let difficulty = hx.difficulty();
                                if difficulty.gt(&best_difficulty) {
                                    best_nonce = nonce;
                                    best_difficulty = difficulty;
                                    best_hash = hx;
                                    // {{ edit_1 }}
                                    if best_difficulty.gt(&*global_best_difficulty.read().unwrap())
                                    {
                                        *global_best_difficulty.write().unwrap() = best_difficulty;
                                    }
                                    // {{ edit_1 }}
                                }
                            }
            

                            // Exit if time has elapsed
                            if nonce % 100 == 0 {
                                let global_best_difficulty =
                                    *global_best_difficulty.read().unwrap();
                                if timer.elapsed().as_secs().ge(&cutoff_time) {
                                    if i.id == 0 {
                                        progress_bar.set_message(format!(
                                            "{}... (difficulty {})",
                                            get_action_name(&resource),
                                            global_best_difficulty,
                                        ));
                                    }
                                    if global_best_difficulty.ge(&min_difficulty) {
                                        // Mine until min difficulty has been met
                                        break;
                                    }
                                } else if i.id == 0 {
                                    progress_bar.set_message(format!(
                                        "{}... (difficulty {}, time {})",
                                        get_action_name(&resource),
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
        config
            .last_reset_at
            .saturating_add(EPOCH_DURATION)
            .saturating_sub(5) // Buffer
            .le(&clock.unix_timestamp)
    }

    pub async fn get_cutoff(&self, coal_proof: Proof, buffer_time: u64) -> u64 {
        let clock = get_clock(&self.rpc_client).await;
        coal_proof
            .last_hash_at
            .saturating_add(60)
            .saturating_sub(buffer_time as i64)
            .saturating_sub(clock.unix_timestamp)
            .max(0) as u64
    }

    pub async fn find_bus(&self, resource: Resource) -> Pubkey {
        // Fetch the bus with the largest balance
        let bus_addresses = match resource {
            Resource::Coal => BUS_ADDRESSES,
            Resource::Ore => ORE_BUS_ADDRESSES,
            Resource::Ingots => SMELTER_BUS_ADDRESSES,
        };
        if let Ok(accounts) = self.rpc_client.get_multiple_accounts(&bus_addresses).await {
            let mut top_bus_balance: u64 = 0;
            let mut top_bus = bus_addresses[0];
            for account in accounts {
                if let Some(account) = account {
                    if let Ok(bus) = Bus::try_from_bytes(&account.data) {
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

fn calculate_multiplier(balance: u64, top_balance: u64) -> f64 {
    1.0 + (balance as f64 / top_balance as f64).min(1.0f64)
}

fn format_duration(seconds: u32) -> String {
    let minutes = seconds / 60;
    let remaining_seconds = seconds % 60;
    format!("{:02}:{:02}", minutes, remaining_seconds)
}

fn get_action_name(resource: &Resource) -> String {
    match resource {
        Resource::Coal => "Mining".to_string(),
        Resource::Ore => "Mining".to_string(),
        Resource::Ingots => "Smelting".to_string(),
    }
}