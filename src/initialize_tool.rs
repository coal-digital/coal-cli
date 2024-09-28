use std::str::FromStr;

use mpl_core::Asset;
use forge_api;
use solana_sdk::{signature::{Keypair, Signer}, transaction::Transaction, pubkey::Pubkey};

use crate::Miner;

impl Miner {
    pub async fn initialize_tool(&self) {
        // Submit initialize tx
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
    }

    pub async fn mint_tool(&self) {
        let blockhash = self.rpc_client.get_latest_blockhash().await.unwrap();
        let mint = Keypair::new();
        let collection = Pubkey::from_str("8wzYMnkuUYtrH8LSxoQJ1umS6dY2K8wjZ3hYCAzk37gd").unwrap();

        let ix = forge_api::instruction::mint(self.signer().pubkey(), collection, mint.pubkey());
        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&self.signer().pubkey()),
            &[&self.signer(), &mint],
            blockhash,
        );
        let res = self.rpc_client.send_and_confirm_transaction(&tx).await;
        println!("{:?}", res);

        let account_info = self.rpc_client.get_account(&mint.pubkey()).await.unwrap();
        let asset = Asset::deserialize(&account_info.data).unwrap();
        println!("New tool crafted:");
        println!("Mint Address: {}", mint.pubkey());
        println!("Tool Name: {}", asset.base.name);
        println!("Tool Owner: {}", asset.base.owner);
    }
}
