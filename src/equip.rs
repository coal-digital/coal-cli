use std::str::FromStr;

use coal_api::{self, consts::{COAL_MAIN_HAND_TOOL, WOOD_MAIN_HAND_TOOL}};
use mpl_core::{Asset, types::UpdateAuthority};
use solana_sdk::{signature::Signer, transaction::Transaction, pubkey::Pubkey};

use crate::{Miner, args::EquipArgs};

impl Miner {
    pub async fn equip(&self, args: EquipArgs) {
        let signer = self.signer();
        let fee_payer = self.fee_payer();
        
        let blockhash = self.rpc_client.get_latest_blockhash().await.unwrap();

        println!("Equipping tool: {}", args.tool);

        let asset_address = Pubkey::from_str(&args.tool).unwrap();
        let asset_data = self.rpc_client.get_account_data(&asset_address).await.unwrap();
        let asset = Asset::from_bytes(&asset_data).unwrap();
        let attribute_list = asset.plugin_list.attributes.unwrap().attributes.attribute_list;
        let resource = attribute_list.iter().find(|attr| attr.key == "resource").unwrap().value.clone();
        let seed = match resource.as_str() {
            "coal" => COAL_MAIN_HAND_TOOL,
            "wood" => WOOD_MAIN_HAND_TOOL,
            _ => panic!("Invalid resource"),
        };
        let collection_address = match asset.base.update_authority {
            UpdateAuthority::Collection(address) => address,
            _ => panic!("Invalid update authority"),
        };

        let ix = coal_api::instruction::equip(
            signer.pubkey(), 
            signer.pubkey(),
            fee_payer.pubkey(),
            asset_address,
            collection_address,
            seed
        );
        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&self.signer().pubkey()),
            &[&self.signer()],
            blockhash,
        );
        let res = self.rpc_client.send_and_confirm_transaction(&tx).await;
        
        if res.is_err() {
            println!("Failed to equip tool: {:?}", res.err().unwrap());
            return;
        }
        
        println!("{:?}", res);
        println!("Tool equipped successfully!");
    }
}

// [
//     "Program EG67mGGTxMGuPxDLWeccczVecycmpj2SokzpWeBoGVTf invoke [1]", 
//     "Program CoREENxT6tW1HoK8ypY1SxRMZTcVPm7R94rH4PZNhX7d invoke [2]", 
//     "Program log: Instruction: Transfer", 
//     "Program log: programs/mpl-core/src/state/asset.rs:284:Approve", 
//     "Program CoREENxT6tW1HoK8ypY1SxRMZTcVPm7R94rH4PZNhX7d consumed 14853 of 195525 compute units", 
//     "Program CoREENxT6tW1HoK8ypY1SxRMZTcVPm7R94rH4PZNhX7d success", 
//     "Program log: durability: 100", 
//     "Program log: multiplier: 70", 
//     "Program log: Equipping wood tool", 
//     "Program EG67mGGTxMGuPxDLWeccczVecycmpj2SokzpWeBoGVTf consumed 40786 of 200000 compute units", 
//     "Program EG67mGGTxMGuPxDLWeccczVecycmpj2SokzpWeBoGVTf failed: Invalid account owner"
// ]