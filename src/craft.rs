use std::str::FromStr;

use forge_api;
use solana_sdk::{signature::{Keypair, Signer}, transaction::Transaction, pubkey::Pubkey};

use crate::Miner;

impl Miner {
    pub async fn craft(&self) {
        let blockhash = self.rpc_client.get_latest_blockhash().await.unwrap();
        let mint = Keypair::new();
        let collection = Pubkey::from_str("FULWiC87wVBemZEreuq6rEw65kMDztaz7qtZHVRND9kr").unwrap();

        let ix = forge_api::instruction::mint(self.signer().pubkey(), collection, mint.pubkey());
        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&self.signer().pubkey()),
            &[&self.signer(), &mint],
            blockhash,
        );
        let res = self.rpc_client.send_and_confirm_transaction(&tx).await;
        println!("{:?}", res);
        println!("New tool crafted:");
        println!("To equip the tool, use command: coal equip --tool {:?}", mint.pubkey());
    }
}
