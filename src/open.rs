use solana_sdk::{instruction::Instruction, signature::Signer};

use crate::{
    send_and_confirm::ComputeBudget,
    utils::{ore_proof_pubkey, proof_pubkey},
    Miner
};

impl Miner {
    pub async fn open(&self) {
        // Return early if miner is already registered
        let signer = self.signer();
        let fee_payer = self.fee_payer();

        let mut ix: Vec<Instruction> = vec![];

        let ore_proof_address = ore_proof_pubkey(signer.pubkey());
        if self.rpc_client.get_account(&ore_proof_address).await.is_err() {
            println!("Generating ORE challenge...");
            ix.push(coal_api::instruction::open_ore(signer.pubkey(), signer.pubkey(), fee_payer.pubkey()));
        }

        let proof_address = proof_pubkey(signer.pubkey());
        if self.rpc_client.get_account(&proof_address).await.is_err() {
            println!("Generating COAL challenge...");
            ix.push(coal_api::instruction::open(signer.pubkey(), signer.pubkey(), fee_payer.pubkey()));
        }

        // Sign and send transaction.        
        self.send_and_confirm(&ix, ComputeBudget::Fixed(800_000), false)
            .await
            .ok();
    }
}
