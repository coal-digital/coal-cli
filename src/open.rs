
use solana_sdk::{instruction::Instruction, signature::Signer};
use coal_api;
use ore_api;
use smelter_api;

use crate::{
    send_and_confirm::ComputeBudget,
    utils::{Resource, get_proof, proof_pubkey, get_resource_name},
    Miner
};

impl Miner {
    pub async fn open(&self, resource: Resource) {
        // Return early if miner is already registered
        let signer = self.signer();
        let fee_payer = self.fee_payer();

        let proof_address = proof_pubkey(signer.pubkey(), resource.clone());
        if self.rpc_client.get_account(&proof_address).await.is_ok() {
            return;
        }

        // Sign and send transaction.
        println!("Generating {} challenge...", get_resource_name(&resource));
        let ix = match resource {
            Resource::Coal => coal_api::instruction::open_coal(signer.pubkey(), signer.pubkey(), fee_payer.pubkey()),
            Resource::Wood => coal_api::instruction::open_wood(signer.pubkey(), signer.pubkey(), fee_payer.pubkey()),
            Resource::Ore => ore_api::instruction::open(signer.pubkey(), signer.pubkey(), fee_payer.pubkey()),
            Resource::Ingots => smelter_api::instruction::open(signer.pubkey(), signer.pubkey(), fee_payer.pubkey()),
            _ => panic!("Resource not supported")
        };
        
        self.send_and_confirm(&[ix], ComputeBudget::Fixed(400_000), false)
            .await
            .ok();
    }

    pub async fn open_merged(&self) -> Result<bool, &str> {
        // Return early if miner is already registered
        let signer = self.signer();
        let fee_payer = self.fee_payer();

        let mut compute_budget = 200_000;
        let mut ix: Vec<Instruction> = vec![];

        let coal_proof_address = proof_pubkey(signer.pubkey(), Resource::Coal);
        let ore_proof_address = proof_pubkey(signer.pubkey(), Resource::Ore);

        let (coal_proof_result, ore_proof_result) = tokio::join!(
            self.rpc_client.get_account(&coal_proof_address),
            self.rpc_client.get_account(&ore_proof_address)
        );

        // For merged mining we need to ensure both are closed if the proofs are not already merged
        if ore_proof_result.is_ok() && coal_proof_result.is_ok() {
            let (coal_proof, ore_proof) = tokio::join!(
                get_proof(&self.rpc_client, &Resource::Coal, coal_proof_address),
                get_proof(&self.rpc_client, &Resource::Ore, ore_proof_address)
            );

            if coal_proof.last_hash().eq(&ore_proof.last_hash()) && coal_proof.challenge().eq(&ore_proof.challenge()) {
                // Proofs are already merged
                return Ok(true);
            }
        }

        // Close the proofs if they do not match and reopen them            
        if coal_proof_result.is_ok() || ore_proof_result.is_ok() {
            return Err("Please close your ORE and COAL accounts before opening a merged account.");
        }

        println!("Opening COAL account...");
        compute_budget += 200_000;
        ix.push(coal_api::instruction::open_coal(signer.pubkey(), signer.pubkey(), fee_payer.pubkey()));
        println!("Opening ORE account...");
        ix.push(ore_api::instruction::open(signer.pubkey(), signer.pubkey(), fee_payer.pubkey()));

        // Sign and send transaction.        
        self.send_and_confirm(&ix, ComputeBudget::Fixed(compute_budget), false)
            .await
            .expect("Failed to open account(s)");

    
        Ok(true)
    }
}
