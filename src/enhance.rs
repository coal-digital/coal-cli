use std::sync::Arc;
use std::str::FromStr;
use tokio::time::{sleep, Duration};

use forge_api::{
    consts::{CHROMIUM_MINT_ADDRESS, ENHANCER_SEED},
    state::Enhancer,
};
use mpl_core::{Asset, types::UpdateAuthority};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_rpc_client::spinner;
use solana_sdk::{signature::{Keypair, Signer}, pubkey::Pubkey, transaction::Transaction};

use forge_utils::AccountDeserialize;
use crate::{Miner, args::{EnhanceArgs, EquipArgs}, send_and_confirm::ComputeBudget, utils::ask_confirm};

fn get_enhancer_address(signer: &Pubkey, asset: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[ENHANCER_SEED, signer.as_ref(), asset.as_ref()], &forge_api::id()).0
}

async fn get_enhancer(client: &RpcClient, signer: &Pubkey, asset: &Pubkey) -> Option<Enhancer> {
    let address = get_enhancer_address(&signer, &asset);
    println!("Enhancer address: {}", address);
    let account_data = client.get_account_data(&address).await;

    if let Ok(account_data) = account_data {
        Some(*Enhancer::try_from_bytes(&account_data).unwrap())
    } else {
        None
    }
    
}

impl Miner {
    pub async fn enhance(&self, args: EnhanceArgs) {
        let signer = self.signer();
        let new_mint: Keypair = Keypair::new();

        println!("Enhancing tool: {}", args.tool);

        let asset_address = Pubkey::from_str(&args.tool).unwrap();
        let asset_data = self.rpc_client.get_account_data(&asset_address).await.unwrap();
        let asset = Asset::from_bytes(&asset_data).unwrap();
        let durability = asset.plugin_list.attributes.unwrap().attributes.attribute_list.iter().find(|attr| attr.key == "durability").unwrap().value.parse::<f64>().unwrap();
        let collection_address = match asset.base.update_authority {
            UpdateAuthority::Collection(address) => address,
            _ => panic!("Invalid update authority"),
        };
        println!("Durability: {}", durability);
        let required_burn_amount = durability / 1000.0;

        let chromium_tokens = spl_associated_token_account::get_associated_token_address(
            &signer.pubkey(),
            &CHROMIUM_MINT_ADDRESS,
        );

        let tokens = self
            .rpc_client
            .get_token_account(&chromium_tokens)
            .await;

        match tokens {
            Ok(token) => {
                let amount = token.unwrap().token_amount.ui_amount_string.parse::<f64>().unwrap();
                println!("You have {:?} Chromium tokens", amount);
                println!("You need {:?} Chromium tokens to enhance this tool", required_burn_amount);
                if amount.le(&required_burn_amount) {
                    println!("You don't have enough Chromium tokens to enhance this tool");
                    // return;
                }
            }
            Err(e) => {
                println!("Chromium tokens not found: {:?}", e);
                return;
            }
        }


        let mut enhancer = get_enhancer(&self.rpc_client, &signer.pubkey(), &asset_address).await;

        if enhancer.is_none() {
            let ix = forge_api::instruction::init_enhance(
                signer.pubkey(), 
                asset_address,
            );
            let res = self.send_and_confirm(&[ix], ComputeBudget::Fixed(100_000), false).await;
            if res.is_err() {
                println!("Failed to initialize chromium enhancer: {:?}", res);
                return;
            }
            enhancer = get_enhancer(&self.rpc_client, &signer.pubkey(), &asset_address).await;
        }

        let target_slot = enhancer.unwrap().slot;

        let progress_bar = Arc::new(spinner::new_progress_bar());
        progress_bar.set_message(format!("Waiting for slot {}...", target_slot));

        loop {
            match self.rpc_client.get_slot().await {
                Ok(current_slot) => {
                    if current_slot >= target_slot {
                        progress_bar.finish_with_message(format!("Target slot {} reached", target_slot));

                        let blockhash = self.rpc_client.get_latest_blockhash().await.unwrap();
                        let ix = forge_api::instruction::enhance(
                            signer.pubkey(), 
                            asset_address,
                            new_mint.pubkey(),
                            collection_address,
                        );
                        let tx = Transaction::new_signed_with_payer(
                            &[ix],
                            Some(&self.signer().pubkey()),
                            &[&self.signer(), &new_mint],
                            blockhash,
                        );
                        let res = self.rpc_client.send_and_confirm_transaction(&tx).await;
                        if res.is_err() {
                            progress_bar.finish_with_message(format!("Failed to finalize enhancement: {:?}", res));
                            return;
                        }
                        progress_bar.finish_with_message(format!("Tool enhanced successfully! New tool address: {}", new_mint.pubkey()));
                        let asset_data = self.rpc_client.get_account_data(&new_mint.pubkey()).await.unwrap();
                        let asset = Asset::from_bytes(&asset_data).unwrap();
                        let multiplier = asset.plugin_list.attributes.unwrap().attributes.attribute_list.iter().find(|attr| attr.key == "multiplier").unwrap().value.parse::<f64>().unwrap();
                        println!("You minted a {}x multiplier!", multiplier / 100.0);
                        break;
                    }
                    sleep(Duration::from_millis(400)).await;
                },
                Err(e) => {
                    progress_bar.finish_with_message(format!("Failed to get current slot {}...", e));
                    sleep(Duration::from_secs(400)).await;
                }
            }
        }

        if !ask_confirm(
            format!(
                "\nWould you like to equip the enhanced pickaxe? [Y/n]",
            )
            .as_str(),
        ) {
            println!("To equip the tool, use command: coal equip --tool {:?}", new_mint.pubkey());
            return;
        }
        
        self.equip(EquipArgs {
            tool: new_mint.pubkey().to_string(),
        }).await;
    }
}