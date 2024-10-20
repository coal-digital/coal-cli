use std::str::FromStr;

use solana_program::pubkey::Pubkey;
use solana_sdk::signature::Signer;
use coal_api;
use smelter_api;
use ore_api;

use crate::{
    args::StakeArgs, cu_limits::CU_LIMIT_CLAIM, send_and_confirm::ComputeBudget,
    utils::{amount_f64_to_u64, get_resource_from_str, Resource, get_resource_mint}, Miner,
};

impl Miner {
    pub async fn stake(&self, args: StakeArgs) {
        // Get signer
        let signer = self.signer();
        let resource = get_resource_from_str(&args.resource);
        let mint = get_resource_mint(&resource);
        
        let sender = match args.token_account {
            Some(address) => {
                Pubkey::from_str(&address).expect("Failed to parse token account address")
            }
            None => spl_associated_token_account::get_associated_token_address(
                &signer.pubkey(),
                &mint,
            ),
        };

        // Get token account
        let Ok(Some(token_account)) = self.rpc_client.get_token_account(&sender).await else {
            println!("Failed to fetch token account");
            return;
        };

        // Parse amount
        let amount: u64 = if let Some(amount) = args.amount {
            amount_f64_to_u64(amount)
        } else {
            u64::from_str(token_account.token_amount.amount.as_str())
                .expect("Failed to parse token balance")
        };

        // Send tx
        let ix = match resource {
            Resource::Coal => coal_api::instruction::stake_coal(signer.pubkey(), sender, amount),
            Resource::Wood => coal_api::instruction::stake_wood(signer.pubkey(), sender, amount),
            Resource::Ingots => smelter_api::instruction::stake(signer.pubkey(), sender, amount),
            Resource::Ore => ore_api::instruction::stake(signer.pubkey(), sender, amount),
            _ => panic!("No staking for resource")
        };
        self.send_and_confirm(&[ix], ComputeBudget::Fixed(CU_LIMIT_CLAIM), false)
            .await
            .ok();
    }
}
