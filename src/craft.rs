use forge_api;
use solana_sdk::{signature::{Keypair, Signer}, transaction::Transaction, pubkey::Pubkey};
use solana_program::pubkey;

use crate::{Miner, utils::ask_confirm, args::EquipArgs};

const PICKAXE_COLLECTION: Pubkey = pubkey!("5h2VTfNMgNzWoQaFrjqbvAEQjd5RzYom9iKiTPbzUFXk");

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
