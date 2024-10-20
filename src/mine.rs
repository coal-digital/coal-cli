use std::{sync::Arc, sync::RwLock, time::Instant};
use colored::*;
use drillx::{
    equix::{self},
    Hash, Solution,
};
use coal_api::{consts::*, state::Bus};
use coal_utils::AccountDeserialize;
use ore_api;
use rand::Rng;
use solana_program::{pubkey::Pubkey, instruction::Instruction};
use solana_rpc_client::spinner;
use solana_sdk::signer::Signer;
use tokio;


use crate::{
    args::MineArgs,
    send_and_confirm::ComputeBudget,
    utils::{
        Resource, ConfigType,
        amount_u64_to_string,
        get_clock, get_config,
        get_updated_proof_with_authority, 
        proof_pubkey, get_resource_from_str, get_resource_name, 
        get_resource_bus_addresses, get_tool_pubkey, get_config_pubkey, 
        deserialize_config, deserialize_tool, ToolType,
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
                self.process_mine_merged(args).await;
            }
            "none" => {
                self.process_mine(args).await;
            }
            _ => {
                println!("{} {} \"{}\" {}", "ERROR".bold().red(), "Argument value", merged, "not recognized");
                return;
            }
        }
    }

    async fn process_mine(&self, args: MineArgs) {
        let resource = get_resource_from_str(&args.resource);
        let signer = self.signer();

        // Check if resource is valid
        if resource == Resource::Ingots {
            println!("{} {}", "ERROR".bold().red(), "Use smelt command for ingots");
            return;
        }

        println!("{} {} {} {}", "INFO".bold().green(), "Started", get_resource_name(&resource), get_action_name(&resource));

        self.open(resource.clone()).await;

        // Check num threads
        self.check_num_cores(args.cores);

        // Start mining loop
        let mut last_hash_at = 0;
        let mut last_balance = 0;
        
        loop {
            // Fetch coal_proof
            let config_address = get_config_pubkey(&resource);
            let tool_address = get_tool_pubkey(signer.pubkey(), &resource);
            
            let accounts = match resource {
                Resource::Coal => self.rpc_client.get_multiple_accounts(&[config_address, tool_address]).await.unwrap(),
                Resource::Wood => self.rpc_client.get_multiple_accounts(&[config_address, tool_address]).await.unwrap(),
                _ => self.rpc_client.get_multiple_accounts(&[config_address]).await.unwrap(),
            };
            
            let config = deserialize_config(&accounts[0].as_ref().unwrap().data, &resource);
            

            let mut tool: Option<ToolType> = None;
            
            if accounts.len() > 1 && accounts[1].as_ref().is_some() {
                tool = Some(deserialize_tool(&accounts[1].as_ref().unwrap().data, &resource));
            }

            let proof = get_updated_proof_with_authority(&self.rpc_client, &resource, signer.pubkey(), last_hash_at).await;

            let top_balance = config.top_balance();
            let min_difficulty = config.min_difficulty();
            
            println!(
                "\n\nStake: {} {}\n{}  Multiplier: {:12}x",
                amount_u64_to_string(proof.balance()),
                get_resource_name(&resource),
                if last_hash_at.gt(&0) {
                    format!(
                        "  Change: {} {}\n",
                        amount_u64_to_string(proof.balance().saturating_sub(last_balance)),
                        get_resource_name(&resource)
                    )
                } else {
                    "".to_string()
                },
                calculate_multiplier(proof.balance(), top_balance, tool)
            );
            

            last_hash_at = proof.last_hash_at();
            last_balance = proof.balance();

            // Calculate cutoff time
            let duration = match resource {
                Resource::Coal => ONE_MINUTE,
                Resource::Ore => ONE_MINUTE,
                Resource::Wood => ONE_MINUTE,
                _ => 0,
            };
            let cutoff_time = self.get_cutoff(proof.last_hash_at(), duration, args.buffer_time).await;

            // Run drillx
            let solution = Self::find_hash_par(proof.challenge(), cutoff_time, args.cores, min_difficulty as u32, &resource)
                .await;


            // Build instruction set
            let mut compute_budget = 500_000;
            let mut ixs: Vec<Instruction> = vec![];

            match resource {
                Resource::Coal => {
                    ixs.push(ore_api::instruction::auth(proof_pubkey(signer.pubkey(), Resource::Ore)));
                    ixs.push(coal_api::instruction::auth(proof_pubkey(signer.pubkey(), Resource::Coal)));
                },
                Resource::Wood => {
                    ixs.push(coal_api::instruction::auth(proof_pubkey(signer.pubkey(), Resource::Wood)));
                },
                Resource::Ore => {
                    ixs.push(ore_api::instruction::auth(proof_pubkey(signer.pubkey(), Resource::Ore)));
                },
                _ => {
                    return;
                }
            }

            // Reset if needed
            let config = get_config(&self.rpc_client, &resource).await;
            if self.should_reset(config).await {
                compute_budget += 100_000;

                match resource {
                    Resource::Coal => {
                        ixs.push(coal_api::instruction::reset_coal(signer.pubkey()));
                    },
                    Resource::Wood => {
                        ixs.push(coal_api::instruction::reset_wood(signer.pubkey()));
                    },
                    _ => {}
                }
            }

            // Build mine ix
            match resource {
                Resource::Coal => {
                    ixs.push(coal_api::instruction::mine_coal(
                        signer.pubkey(),
                        signer.pubkey(),
                        self.find_bus(Resource::Coal).await,
                        solution,
                    ));
                },
                Resource::Ore => {
                    ixs.push(ore_api::instruction::mine(
                        signer.pubkey(),
                        signer.pubkey(),
                        self.find_bus(Resource::Ore).await,
                        solution,
                    ));
                },
                Resource::Wood => {
                    ixs.push(coal_api::instruction::chop_wood(
                        signer.pubkey(),
                        signer.pubkey(),
                        self.find_bus(Resource::Wood).await,
                        solution,
                    ));
                },
                _ => {
                    return;
                }
            }

            // Submit transactions
            self.send_and_confirm(&ixs, ComputeBudget::Fixed(compute_budget), false).await.ok();
        }
    }

    async fn process_mine_merged(&self, args: MineArgs) {
        // Open accounts, if needed.
        let result = self.open_merged().await;
        
        if result.is_err() {
            println!("{} {}", "ERROR".bold().red(), result.err().unwrap());
            return;
        }

        let signer = self.signer();

        // Check num threads
        self.check_num_cores(args.cores);

        // Start mining loop
        let mut last_coal_hash_at = 0;
        let mut last_coal_balance = 0;
        let mut last_ore_hash_at = 0;
        let mut last_ore_balance = 0;
        loop {
            let coal_config_address = get_config_pubkey(&Resource::Coal);
            let ore_config_address = get_config_pubkey(&Resource::Ore);
            let tool_address = get_tool_pubkey(signer.pubkey(), &Resource::Coal);

            let accounts = self.rpc_client.get_multiple_accounts(&[coal_config_address, ore_config_address, tool_address]).await.unwrap();

            let coal_config = deserialize_config(&accounts[0].as_ref().unwrap().data, &Resource::Coal);
            let ore_config = deserialize_config(&accounts[1].as_ref().unwrap().data, &Resource::Ore);
            let tool: Option<ToolType> = if accounts[2].as_ref().is_some() {
                Some(deserialize_tool(&accounts[2].as_ref().unwrap().data, &Resource::Coal))
            } else {
                None
            };
            
            // Fetch coal_proof
            let (coal_proof, ore_proof) = tokio::join!(
                // TODO: reduce the number of requests!
                get_updated_proof_with_authority(&self.rpc_client, &Resource::Coal, signer.pubkey(), last_coal_hash_at),
                get_updated_proof_with_authority(&self.rpc_client, &Resource::Ore, signer.pubkey(), last_ore_hash_at)
            );

            let coal_top_balance = coal_config.top_balance();
            let ore_top_balance = ore_config.top_balance();
            let coal_min_difficulty = coal_config.min_difficulty();
            let ore_min_difficulty = ore_config.min_difficulty();

            println!(
                "\n\nStake: {} COAL\n{}  Multiplier: {:12}x",
                amount_u64_to_string(coal_proof.balance()),
                if last_coal_hash_at.gt(&0) {
                    format!(
                        "  Change: {} COAL\n",
                        amount_u64_to_string(coal_proof.balance().saturating_sub(last_coal_balance))
                    )
                } else {
                    "".to_string()
                },
                calculate_multiplier(coal_proof.balance(), coal_top_balance, tool)
            );
            println!(
                "Stake: {} ORE\n{}  Multiplier: {:12}x",
                amount_u64_to_string(ore_proof.balance()),
                if last_ore_hash_at.gt(&0) {
                    format!(
                        "  Change: {} ORE\n",
                        amount_u64_to_string(ore_proof.balance().saturating_sub(last_ore_balance))
                    )
                } else {
                    "".to_string()
                },
                calculate_multiplier(ore_proof.balance(), ore_top_balance, None)
            );
            

            last_coal_hash_at = coal_proof.last_hash_at();
            last_coal_balance = coal_proof.balance();
            last_ore_hash_at = ore_proof.last_hash_at();
            last_ore_balance = ore_proof.balance();

            // Calculate cutoff time
            let cutoff_time = self.get_cutoff(coal_proof.last_hash_at(), ONE_MINUTE, args.buffer_time).await;

            // Run drillx
            let min_difficulty = coal_min_difficulty.max(ore_min_difficulty);
            let solution = Self::find_hash_par(coal_proof.challenge(), cutoff_time, args.cores, min_difficulty as u32, &Resource::Coal)
                .await;


            let mut compute_budget = 950_000;
            // Build instruction set
            let mut ixs = vec![
                ore_api::instruction::auth(proof_pubkey(signer.pubkey(), Resource::Ore)),
                coal_api::instruction::auth(proof_pubkey(signer.pubkey(), Resource::Coal)),
            ];

            // Reset if needed
            let coal_config = get_config(&self.rpc_client, &Resource::Coal).await;
            if self.should_reset(coal_config).await {
                compute_budget += 100_000;
                ixs.push(coal_api::instruction::reset_coal(signer.pubkey()));
            }

            // Build mine ix
            ixs.push(ore_api::instruction::mine(
                signer.pubkey(),
                signer.pubkey(),
                self.find_bus(Resource::Ore).await,
                solution,
            ));
            ixs.push(coal_api::instruction::mine_coal(
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
        challenge: [u8; 32],
        cutoff_time: u64,
        cores: u64,
        min_difficulty: u32,
        resource: &Resource,
    ) -> Solution {
        // Dispatch job to each thread
        let progress_bar = Arc::new(spinner::new_progress_bar());
        let global_best_difficulty = Arc::new(RwLock::new(0u32));
        progress_bar.set_message(get_action_name(&resource));
        let core_ids = core_affinity::get_core_ids().unwrap();
        let handles: Vec<_> = core_ids
            .into_iter()
            .map(|i| {
                let global_best_difficulty = Arc::clone(&global_best_difficulty);
                std::thread::spawn({
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

    pub async fn should_reset(&self, config: ConfigType) -> bool {
        let clock = get_clock(&self.rpc_client).await;
        match config {
            ConfigType::General(config) => config.last_reset_at
                .saturating_add(COAL_EPOCH_DURATION)
                .saturating_sub(5) // Buffer
                .le(&clock.unix_timestamp),
            ConfigType::Wood(config) => config.last_reset_at
                .saturating_add(WOOD_EPOCH_DURATION)
                .saturating_sub(5) // Buffer
                .le(&clock.unix_timestamp),
        }
    }

    pub async fn get_cutoff(&self, last_hash_at: i64, duration: i64, buffer_time: u64) -> u64 {
        let clock = get_clock(&self.rpc_client).await;
        last_hash_at
            .saturating_add(duration)
            .saturating_sub(buffer_time as i64)
            .saturating_sub(clock.unix_timestamp)
            .max(0) as u64
    }

    pub async fn find_bus(&self, resource: Resource) -> Pubkey {
        // Fetch the bus with the largest balance
        let bus_addresses = get_resource_bus_addresses(&resource);
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

fn calculate_multiplier(balance: u64, top_balance: u64, tool: Option<ToolType>) -> f64 {
   let base_multiplier = 1.0 + (balance as f64 / top_balance as f64).min(1.0f64);

    match tool {
        Some(tool) => {
            if tool.durability().gt(&0) {
                let tool_multiplier = 1.0 + (tool.multiplier() as f64 / 100.0);
                return base_multiplier * tool_multiplier;
            }
            return base_multiplier;
        }
        None => base_multiplier,
    }
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
        Resource::Wood => "Chopping".to_string(),
        Resource::Chromium => "Reprocessing".to_string(),
    }
}