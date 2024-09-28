use std::str::FromStr;

use coal_api;
use solana_sdk::{signature::Signer, transaction::Transaction, pubkey::Pubkey};

use crate::Miner;

impl Miner {
    pub async fn equip(&self) {
        let signer = self.signer();
        let fee_payer = self.fee_payer();

        let blockhash = self.rpc_client.get_latest_blockhash().await.unwrap();
        let asset = Pubkey::from_str("GW5tTHsv1wX54ux9oqGjKnbaBVkqDsKFUmKhHmfNhz1m").unwrap();
		let collection = Pubkey::from_str("8wzYMnkuUYtrH8LSxoQJ1umS6dY2K8wjZ3hYCAzk37gd").unwrap();

        let ix = coal_api::instruction::equip(signer.pubkey(), signer.pubkey(), fee_payer.pubkey(), asset, collection);
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
