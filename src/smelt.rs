use coal_api::consts::ONE_MINUTE;
use solana_sdk::signer::Signer;

use smelter_api::consts::MAX_EFFICIENCY_BONUS_PERCENTAGE;
use crate::{
    args::SmeltArgs,
    send_and_confirm::ComputeBudget,
    utils::{
        Resource,
        amount_u64_to_string,
        get_config,
        get_updated_proof_with_authority,
        proof_pubkey,
    },
    Miner,
};

impl Miner {
    pub async fn smelt(&self, args: SmeltArgs) {
        let signer = self.signer();
        self.open(Resource::Ingots).await;

        // Check num threads
        self.check_num_cores(args.cores);

        // Start smelting loop
        let mut last_hash_at = 0;
        let mut last_balance = 0;
        let mut last_coal_balance: u64 = 0;
        let mut last_ore_balance: u64 = 0;

        let coal_token_account_address = spl_associated_token_account::get_associated_token_address(
            &signer.pubkey(),
            &coal_api::consts::COAL_MINT_ADDRESS,
        );
        let ore_token_account_address = spl_associated_token_account::get_associated_token_address(
            &signer.pubkey(),
            &ore_api::consts::MINT_ADDRESS,
        );

        loop {
            // Fetch proof
            let (config, proof, coal_token_account, ore_token_account) = tokio::join!(
                get_config(&self.rpc_client, &Resource::Ingots),
                get_updated_proof_with_authority(&self.rpc_client, &Resource::Ingots, signer.pubkey(), last_hash_at),
                self.rpc_client.get_token_account(&coal_token_account_address),
                self.rpc_client.get_token_account(&ore_token_account_address),
            );

            let coal_token_account = match coal_token_account {
                Ok(Some(coal_token_account)) => coal_token_account,
                Err(e) => {
                    println!("Error fetching coal token account: {:?}", e);
                    return;
                }
                Ok(None) => {
                    println!("No coal token account found");
                    return;
                }
            };

            let ore_token_account = match ore_token_account {
                Ok(Some(ore_token_account)) => ore_token_account,
                Err(e) => {
                    println!("Error fetching coal token account: {:?}", e);
                    return;
                }
                Ok(None) => {
                    println!("No coal token account found");
                    return;
                }
            };

            let coal_balance = coal_token_account.token_amount.amount.parse::<u64>().unwrap_or(0);
            let ore_balance = ore_token_account.token_amount.amount.parse::<u64>().unwrap_or(0);

            if coal_balance == 0 {
                println!("Not enough COAL to smelt, foreman");
                return;
            }

            if ore_balance == 0 {
                println!("Not enough ORE to smelt, foreman");
                return;
            }
            
            println!(
                "\n\nStake: {} INGOT\n{}  Multiplier: {:12}x\n  Efficiency Bonus: {:12}%\n",
                amount_u64_to_string(proof.balance()),
                if last_hash_at.gt(&0) {
                    format!(
                        "  Change: {} INGOT\n  Coal Burn: {} COAL\n  Ore Wrapped: {} ORE\n",
                        amount_u64_to_string(proof.balance().saturating_sub(last_balance)),
                        amount_u64_to_string(last_coal_balance.saturating_sub(coal_balance)),
                        amount_u64_to_string(last_ore_balance.saturating_sub(ore_balance))
                    )
                } else {
                    "".to_string()
                },
                calculate_multiplier(proof.balance(), config.top_balance()),
                calculate_discount(proof.balance(), config.top_balance())
            );

            last_hash_at = proof.last_hash_at();
            last_balance = proof.balance();
            last_coal_balance = coal_balance;
            last_ore_balance = ore_balance;

            // Calculate cutoff time
            let cutoff_time = self.get_cutoff(proof.last_hash_at(), ONE_MINUTE, args.buffer_time).await;

            // Run drillx
            let solution = Self::find_hash_par(proof.challenge(), cutoff_time, args.cores, config.min_difficulty() as u32, &Resource::Ingots).await;


            let mut compute_budget = 500_000;
            // Build instruction set
            let mut ixs = vec![
                smelter_api::instruction::auth(proof_pubkey(signer.pubkey(), Resource::Ingots)),
            ];

            // Reset if needed
            let config = get_config(&self.rpc_client, &Resource::Ingots).await;
            
            if self.should_reset(config).await {
                compute_budget += 100_000;
                ixs.push(smelter_api::instruction::reset(signer.pubkey()));
            }

            // Build mine ix
            ixs.push(smelter_api::instruction::smelt(
                signer.pubkey(),
                signer.pubkey(),
                self.find_bus(Resource::Ingots).await,
                solution,
            ));

            // Submit transactions
            self.send_and_confirm(&ixs, ComputeBudget::Fixed(compute_budget), false).await.ok();
        }
    }
}

fn calculate_multiplier(balance: u64, top_balance: u64) -> f64 {
    1.0 + (balance as f64 / top_balance as f64).min(1.0f64)
}

fn calculate_discount(balance: u64, top_balance: u64) -> f64 {
    ((balance as f64 / top_balance as f64).min(1.0) * MAX_EFFICIENCY_BONUS_PERCENTAGE) * 100.0
}
