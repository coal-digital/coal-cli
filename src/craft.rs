use forge_api;
use coal_api::consts::FORGE_PICKAXE_COLLECTION;
use solana_sdk::{signature::{Keypair, Signer}, transaction::Transaction};

use crate::{Miner, utils::ask_confirm, args::EquipArgs};

impl Miner {
    pub async fn craft(&self) {
        let blockhash = self.rpc_client.get_latest_blockhash().await.unwrap();
        let mint: Keypair = Keypair::new();

        let ix = forge_api::instruction::mint(self.signer().pubkey(), FORGE_PICKAXE_COLLECTION, mint.pubkey());
        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&self.signer().pubkey()),
            &[&self.signer(), &mint],
            blockhash,
        );
        let res = self.rpc_client.send_and_confirm_transaction(&tx).await;
        
        if res.is_err() {
            println!("Failed to craft pickaxe: {:?}", res.err().unwrap());
            return;
        } else {
            println!("{:?}", res);
        }

        println!("Pickaxe crafted!");

        if !ask_confirm(
            format!(
                "\nWould you like to equip the pickaxe? [Y/n]",
            )
            .as_str(),
        ) {
            println!("To equip the tool, use command: coal equip --tool {:?}", mint.pubkey());
            return;
        }

        self.equip(EquipArgs {
            tool: mint.pubkey().to_string(),
        }).await;

    }
}
