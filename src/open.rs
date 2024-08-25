
use solana_sdk::{instruction::Instruction, signature::Signer};

use crate::{
    send_and_confirm::ComputeBudget,
    utils::{Resource, get_proof, proof_pubkey},
    Miner
};

impl Miner {
    pub async fn open(&self, merged: String) -> Result<bool, &str> {
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

        if merged == "ore" {
            // For merged mining we need to ensure both are closed if the proofs are not already merged
            if ore_proof_result.is_ok() && coal_proof_result.is_ok() {
                let (coal_proof, ore_proof) = tokio::join!(
                    get_proof(&self.rpc_client, coal_proof_address),
                    get_proof(&self.rpc_client, ore_proof_address)
                );

                if coal_proof.last_hash.eq(&ore_proof.last_hash) && coal_proof.challenge.eq(&ore_proof.challenge) {
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
            ix.push(coal_api::instruction::open(signer.pubkey(), signer.pubkey(), fee_payer.pubkey()));
            println!("Opening ORE account...");
            ix.push(ore_api::instruction::open(signer.pubkey(), signer.pubkey(), fee_payer.pubkey()));
        } else if coal_proof_result.is_err() {
            println!("Opening COAL account...");
            ix.push(coal_api::instruction::open(signer.pubkey(), signer.pubkey(), fee_payer.pubkey()));
        } else {
            return Ok(true);
        }

        // Sign and send transaction.        
        self.send_and_confirm(&ix, ComputeBudget::Fixed(compute_budget), false)
            .await
            .expect("Failed to open account(s)");

    
        Ok(true)
    }

    pub async fn open_smelter(&self) {
        // Return early if miner is already registered
        let signer = self.signer();
        let fee_payer = self.fee_payer();
        let proof_address = proof_pubkey(signer.pubkey(), Resource::Ingots);
        if self.rpc_client.get_account(&proof_address).await.is_ok() {
            return;
        }

        // Sign and send transaction.
        println!("Generating challenge...");
        let ix = smelter_api::instruction::open(signer.pubkey(), signer.pubkey(), fee_payer.pubkey());
        self.send_and_confirm(&[ix], ComputeBudget::Fixed(400_000), false)
            .await
            .ok();
    }
}
