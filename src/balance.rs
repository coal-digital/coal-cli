use std::str::FromStr;

use solana_program::pubkey::Pubkey;
use solana_sdk::signature::Signer;

use crate::{
    args::BalanceArgs,
    utils::{amount_u64_to_string, get_proof_with_authority, get_resource_name, get_resource_from_str, get_resource_mint, Resource},
    Miner,
};

impl Miner {
    pub async fn balance(&self, args: BalanceArgs) {
        let signer = self.signer();
        let address = if let Some(address) = args.address {
            if let Ok(address) = Pubkey::from_str(&address) {
                address
            } else {
                println!("Invalid address: {:?}", address);
                return;
            }
        } else {
            signer.pubkey()
        };
        let resource = get_resource_from_str(&args.resource);

        let proof = match resource {
            Resource::Chromium => None,
            _ => {
                let proof = get_proof_with_authority(&self.rpc_client, address, &resource).await;
                Some(proof)
            }
        };

        let token_mint: Pubkey = get_resource_mint(&resource);
        let token_account_address = spl_associated_token_account::get_associated_token_address(
            &address,
        &token_mint,
        );
        let token_balance = if let Ok(Some(token_account)) = self
            .rpc_client
            .get_token_account(&token_account_address)
            .await
        {
            token_account.token_amount.ui_amount_string
        } else {
            "0".to_string()
        };
        
        let resource_name = get_resource_name(&resource);

        match proof {
            Some(proof) => {
                println!(
                    "Balance: {} {}\nStake: {} {}",
                    token_balance,
                    resource_name,
                    amount_u64_to_string(proof.balance()),
                    resource_name,
                )
            }
            None => {
                println!("Balance: {} {}", token_balance, resource_name);
            }
        }
    }
}
