use coal_api::{consts::*, state::Tool};
use solana_sdk::{signature::Signer, transaction::Transaction, pubkey::Pubkey};
use mpl_core::{Asset, types::UpdateAuthority};
use coal_utils::AccountDeserialize;

use crate::Miner;

impl Miner {
    pub async fn unequip(&self) {
        let signer = self.signer();
        let fee_payer = self.fee_payer();

        println!("Unequipping tool...");

        let (tool_address, _bump) = Pubkey::find_program_address(&[&COAL_MAIN_HAND_TOOL, signer.pubkey().as_ref()], &coal_api::id());
        let tool_account_info = self.rpc_client.get_account(&tool_address).await.unwrap();

        if tool_account_info.data.is_empty() {
            println!("No tool equipped");
            return;
        }
        
        let tool = Tool::try_from_bytes(&tool_account_info.data).unwrap();
        let asset_data = self.rpc_client.get_account_data(&tool.asset).await.unwrap();
        let asset = Asset::from_bytes(&asset_data).unwrap();
        let collection_address = match asset.base.update_authority {
            UpdateAuthority::Collection(address) => address,
            _ => panic!("Invalid update authority"),
        };
    
        let blockhash = self.rpc_client.get_latest_blockhash().await.unwrap();

        let ix = coal_api::instruction::unequip(signer.pubkey(), signer.pubkey(), fee_payer.pubkey(), tool.asset, collection_address);
        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&self.signer().pubkey()),
            &[&self.signer()],
            blockhash,
        );
        let res = self.rpc_client.send_and_confirm_transaction(&tx).await;
        println!("{:?}", res);
        println!("Unequipped tool: {}", tool.asset);
    }
}