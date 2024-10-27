use std::str::FromStr;

use solana_program::pubkey::Pubkey;
use solana_sdk::{signature::{Keypair, Signer}, transaction::Transaction};
use coal_api::consts::{TREASURY_ADDRESS, WOOD_CONFIG_ADDRESS};
use smelter_api::consts::TREASURY_ADDRESS as SMELTER_TREASURY_ADDRESS;
use forge_api::consts::TREASURY_ADDRESS as FORGE_TREASURY_ADDRESS;

use crate::Miner;

impl Miner {
    pub async fn new_tool(&self) {
        // Submit tx
        let blockhash = self.rpc_client.get_latest_blockhash().await.unwrap();
        let mint = Keypair::new();

        let ix = forge_api::instruction::new(self.signer().pubkey(), mint.pubkey());
        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&self.signer().pubkey()),
            &[&self.signer(), &mint],
            blockhash,
        );
        let res = self.rpc_client.send_and_confirm_transaction(&tx).await;
        println!("{:?}", res);
        println!("New tool initialized: {}", mint.pubkey());
    }

    pub async fn verify(&self) {
        let blockhash = self.rpc_client.get_latest_blockhash().await.unwrap();
        let destination = Pubkey::from_str("3ofYzSZAsEi4y5w1mfcCrMneGEr7rQKUAGfxhLpmZpZa").unwrap();

        let ix = forge_api::instruction::verify(self.signer().pubkey(), destination);
        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&self.signer().pubkey()),
            &[&self.signer()],
            blockhash,
        );
        let res = self.rpc_client.send_and_confirm_transaction(&tx).await;
        println!("{:?}", res);
    }
}
