use std::sync::Arc;
use coal_utils::AccountDeserialize;
use solana_client::nonblocking::rpc_client::RpcClient;
use tokio::time::{sleep, Duration};

use colored::*;
use solana_sdk::signer::Signer;
use solana_program::pubkey::Pubkey;
use solana_rpc_client::spinner;
use coal_api::{consts::REPROCESSOR, state::Reprocessor};


use crate::{
    args::ReprocessArgs,
    send_and_confirm::ComputeBudget,
    Miner, utils::Resource,
};

fn get_reprocessor_address(signer: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[REPROCESSOR, signer.as_ref()], &coal_api::id()).0
}

async fn get_reprocessor(client: &RpcClient, signer: &Pubkey) -> Option<Reprocessor> {
    let address = get_reprocessor_address(&signer);
    let account_data = client.get_account_data(&address).await;

    if let Ok(account_data) = account_data {
        Some(*Reprocessor::try_from_bytes(&account_data).unwrap())
    } else {
        None
    }
    
}

impl Miner {
    pub async fn reprocess(&self, _args: ReprocessArgs) {
		let signer = self.signer();
        println!("{} {}", "INFO".bold().green(), "Reprocessing...");
        // Init ata
        let token_account_pubkey = self.initialize_ata(signer.pubkey(), Resource::Chromium).await;

        let token_account = self.rpc_client.get_token_account(&token_account_pubkey).await.unwrap();
        let initial_balance = token_account.unwrap().token_amount.ui_amount.unwrap();

        let mut reprocessor = get_reprocessor(&self.rpc_client, &signer.pubkey()).await;

        if reprocessor.is_none() {
            let ix = coal_api::instruction::init_reprocess(signer.pubkey());
            self.send_and_confirm(&[ix], ComputeBudget::Fixed(100_000), false).await.ok();
            reprocessor = get_reprocessor(&self.rpc_client, &signer.pubkey()).await;
        }

        let target_slot = reprocessor.unwrap().slot;

        let progress_bar = Arc::new(spinner::new_progress_bar());
        progress_bar.set_message(format!("Waiting for slot {}...", target_slot));
        
        loop {
            match self.rpc_client.get_slot().await {
                Ok(current_slot) => {
                    if current_slot >= target_slot - 1 {
                        // println!("{} {}", "INFO".bold().green(), format!("Target slot {} reached", target_slot));
                        progress_bar.finish_with_message(format!("Target slot {} reached", target_slot));
                        let ix = coal_api::instruction::reprocess(signer.pubkey());
                        self.send_and_confirm(&[ix], ComputeBudget::Fixed(200_000), false).await.ok();
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


        let token_account = self.rpc_client.get_token_account(&token_account_pubkey).await.unwrap();
        let final_balance = token_account.unwrap().token_amount.ui_amount.unwrap();
        println!("{} {}", "INFO".bold().green(), format!("Reprocessed {} Chromium", final_balance - initial_balance));
	}
}