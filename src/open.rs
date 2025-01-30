
use solana_sdk::{instruction::Instruction, signature::Signer};
use coal_api::{sdk::open, consts::MINT_ADDRESS, state::proof_pda};
use ore_api;

use crate::{
    send_and_confirm::ComputeBudget,
    utils::{Resource, get_proof, proof_pubkey, get_resource_name},
    Miner
};

impl Miner {
    pub async fn open(&self) {
        let mint = MINT_ADDRESS;
        let signer = self.signer();
        let fee_payer = self.fee_payer();
        
        let proof_address = proof_pda(mint, signer.pubkey()).0;
        // Return early if miner is already registered
        if self.rpc_client.get_account(&proof_address).await.is_ok() {
            return;
        }

        // Sign and send transaction.
        let ix = coal_api::sdk::open(mint, signer.pubkey(), signer.pubkey(), fee_payer.pubkey());
        
        self.send_and_confirm(&[ix], ComputeBudget::Fixed(400_000), false)
            .await
            .ok();
    }
}
