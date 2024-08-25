use solana_sdk::signer::Signer;

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
        self.open_smelter().await;

        // Check num threads
        self.check_num_cores(args.cores);

        // Start smelting loop
        let mut last_hash_at = 0;
        let mut last_balance = 0;
        loop {
            // Fetch proof
            let config = get_config(&self.rpc_client, Resource::Ingots).await;
            let proof = get_updated_proof_with_authority(&self.rpc_client, signer.pubkey(), last_hash_at, Resource::Ingots).await;

            println!(
                "\n\nStake: {} INGOT\n{}  Multiplier: {:12}x",
                amount_u64_to_string(proof.balance),
                if last_hash_at.gt(&0) {
                    format!(
                        "  Change: {} COAL\n",
                        amount_u64_to_string(proof.balance.saturating_sub(last_balance))
                    )
                } else {
                    "".to_string()
                },
                calculate_multiplier(proof.balance, config.top_balance)
            );

            last_hash_at = proof.last_hash_at;
            last_balance = proof.balance;

            // Calculate cutoff time
            let cutoff_time = self.get_cutoff(proof, args.buffer_time).await;

            // Run drillx_2
            let solution = Self::find_hash_par(proof, cutoff_time, args.cores, config.min_difficulty as u32).await;


            let mut compute_budget = 500_000;
            // Build instruction set
            let mut ixs = vec![
                ore_api::instruction::auth(proof_pubkey(signer.pubkey(), Resource::Ingots)),
            ];

            // Reset if needed
            let config = get_config(&self.rpc_client, Resource::Ingots).await;
            if self.should_reset(config).await {
                compute_budget += 100_000;
                ixs.push(coal_api::instruction::reset(signer.pubkey()));
            }

            // Build mine ix
            ixs.push(smelter_api::instruction::smelt(
                signer.pubkey(),
                signer.pubkey(),
                self.find_bus(false).await,
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