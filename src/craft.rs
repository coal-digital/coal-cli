use forge_api;
use solana_sdk::{signature::{Keypair, Signer}, transaction::Transaction, pubkey::Pubkey};
use solana_program::pubkey;

use crate::Miner;

const PICKAXE_COLLECTION: Pubkey = pubkey!("4puH69674RwS65N2XuYbbjcVn3wUpoJdjbY4ZceZKqcc");

impl Miner {
    pub async fn craft(&self) {
        let blockhash = self.rpc_client.get_latest_blockhash().await.unwrap();
        let mint: Keypair = Keypair::new();

        let ix = forge_api::instruction::mint(self.signer().pubkey(), PICKAXE_COLLECTION, mint.pubkey());
        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&self.signer().pubkey()),
            &[&self.signer(), &mint],
            blockhash,
        );
        let res = self.rpc_client.send_and_confirm_transaction(&tx).await;
        println!("{:?}", res);
        println!("Pickaxe crafted!");
        println!("To equip the tool, use command: coal equip --tool {:?}", mint.pubkey());
    }
}
