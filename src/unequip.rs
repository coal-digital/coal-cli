use std::str::FromStr;

use coal_api::{consts::*, state::Tool};
use solana_sdk::{signature::Signer, transaction::Transaction, pubkey::Pubkey};
use mpl_core::Asset;
use coal_utils::AccountDeserialize;

use crate::Miner;

impl Miner {
    pub async fn unequip(&self) {
        let signer = self.signer();
        let fee_payer = self.fee_payer();

        let (tool_address, _bump) = Pubkey::find_program_address(&[&COAL_TOOL, signer.pubkey().as_ref()], &coal_api::id());
        println!("Tool address: {}", tool_address);
        let tool_account_info = self.rpc_client.get_account(&tool_address).await.unwrap();
        let tool = Tool::try_from_bytes(&tool_account_info.data).unwrap();
        println!("Tool: {:?}", tool);

        let blockhash = self.rpc_client.get_latest_blockhash().await.unwrap();
        let asset_address = Pubkey::from_str("C7iqmiXCyUb6H1hNiJGC9hgBfLMofaTE4qhKCnwonbtS").unwrap();
		let collection_address = Pubkey::from_str("54Raz7fjrBb8bMfE6xJHdUJE9dFhHtnh13ReHgg6bCF5").unwrap();

        let account_info = self.rpc_client.get_account(&asset_address).await.unwrap();
        let asset = Asset::deserialize(&account_info.data).unwrap();

        println!("Transferring asset...");
        println!("Asset: {}", asset.base.name);
        println!("Owner: {}", asset.base.owner);

        let ix = coal_api::instruction::unequip(signer.pubkey(), signer.pubkey(), fee_payer.pubkey(), asset_address, collection_address);
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

// [
//     "Program EG67mGGTxMGuPxDLWeccczVecycmpj2SokzpWeBoGVTf invoke [1]", 
//     "Program CoREENxT6tW1HoK8ypY1SxRMZTcVPm7R94rH4PZNhX7d invoke [2]", 
//     "Program log: Instruction: Transfer", 
//     "Program log: programs/mpl-core/src/state/asset.rs:284:Approve", 
//     "Program CoREENxT6tW1HoK8ypY1SxRMZTcVPm7R94rH4PZNhX7d consumed 14096 of 195295 compute units", 
//     "Program CoREENxT6tW1HoK8ypY1SxRMZTcVPm7R94rH4PZNhX7d success", 
//     "Program log: durability: 1", 
//     "54Raz7fjrBb8bMfE6xJHdUJE9dFhHtnh13ReHgg6bCF5's writable privilege escalated", 
//     "Program EG67mGGTxMGuPxDLWeccczVecycmpj2SokzpWeBoGVTf consumed 57059 of 200000 compute units", 
//     "Program EG67mGGTxMGuPxDLWeccczVecycmpj2SokzpWeBoGVTf failed: Cross-program invocation with unauthorized signer or writable account"
// ]