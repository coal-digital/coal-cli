use coal_api::consts::{TREASURY_ADDRESS, WOOD_CONFIG_ADDRESS};
use smelter_api::consts::TREASURY_ADDRESS as SMELTER_TREASURY_ADDRESS;
use forge_api::consts::TREASURY_ADDRESS as FORGE_TREASURY_ADDRESS;
use solana_sdk::{signature::{Keypair, Signer}, transaction::Transaction};

use crate::Miner;

impl Miner {
    // pub async fn initialize(&self) {
    //     // Return early if program is already initialized
    //     if self.rpc_client.get_account(&TREASURY_ADDRESS).await.is_ok() {
    //         return;
    //     }

    //     // Submit initialize tx
    //     let blockhash = self.rpc_client.get_latest_blockhash().await.unwrap();
    //     let ix = coal_api::instruction::init_coal(self.signer().pubkey());
    //     let tx = Transaction::new_signed_with_payer(
    //         &[ix],
    //         Some(&self.signer().pubkey()),
    //         &[&self.signer()],
    //         blockhash,
    //     );
    //     let res = self.rpc_client.send_and_confirm_transaction(&tx).await;
    //     println!("{:?}", res);
    // }

    // pub async fn initialize_smelter(&self) {
    //     // Return early if program is already initialized
    //     if self.rpc_client.get_account(&SMELTER_TREASURY_ADDRESS).await.is_ok() {
    //         return;
    //     }

    //     // Submit initialize tx
    //     let blockhash = self.rpc_client.get_latest_blockhash().await.unwrap();
    //     let ix = smelter_api::instruction::initialize(self.signer().pubkey());
    //     let tx = Transaction::new_signed_with_payer(
    //         &[ix],
    //         Some(&self.signer().pubkey()),
    //         &[&self.signer()],
    //         blockhash,
    //     );
    //     let res = self.rpc_client.send_and_confirm_transaction(&tx).await;
    //     println!("{:?}", res);
    // }

    // pub async fn initialize_wood(&self) {
    //     // Return early if program is already initialized
    //     if self.rpc_client.get_account(&WOOD_CONFIG_ADDRESS).await.is_ok() {
    //         return;
    //     }

    //     // Submit initialize tx
    //     let blockhash = self.rpc_client.get_latest_blockhash().await.unwrap();
    //     let ix = coal_api::instruction::init_wood(self.signer().pubkey());
    //     let tx = Transaction::new_signed_with_payer(
    //         &[ix],
    //         Some(&self.signer().pubkey()),
    //         &[&self.signer()],
    //         blockhash,
    //     );
    //     let res = self.rpc_client.send_and_confirm_transaction(&tx).await;
    //     println!("{:?}", res);
    // }

    pub async fn initialize_chromium(&self) {
        // Submit initialize tx
        let blockhash = self.rpc_client.get_latest_blockhash().await.unwrap();
        let ix = coal_api::instruction::init_chromium(self.signer().pubkey());
        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&self.signer().pubkey()),
            &[&self.signer()],
            blockhash,
        );
        let res = self.rpc_client.send_and_confirm_transaction(&tx).await;
        println!("{:?}", res);
    }

    pub async fn initialize_forge(&self) {
        // Return early if program is already initialized
        if self.rpc_client.get_account(&FORGE_TREASURY_ADDRESS).await.is_ok() {
            return;
        }

        // Submit initialize tx
        let blockhash = self.rpc_client.get_latest_blockhash().await.unwrap();
        let ix = forge_api::instruction::initialize(self.signer().pubkey());
        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&self.signer().pubkey()),
            &[&self.signer()],
            blockhash,
        );
        let res = self.rpc_client.send_and_confirm_transaction(&tx).await;
        println!("{:?}", res);
    }

    pub async fn new_tool(&self) {
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
        println!("New tool initialized: {}", mint.pubkey());
    }
}
