use std::str::FromStr;

use coal_api::{consts::MINT_ADDRESS, state::proof_pda};
use solana_program::pubkey::Pubkey;
use solana_sdk::signature::Signer;

use crate::{
    args::BalanceArgs,
    utils::{amount_u64_to_string, deserialize_proof},
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

        let mint: Pubkey = MINT_ADDRESS;
        let proof_address = proof_pda(mint, address).0;
        let proof_account = self.rpc_client.get_account(&proof_address).await;

        let token_account_address = spl_associated_token_account::get_associated_token_address(
            &address,
            &mint,
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

        match proof_account {
            Ok(account_info) => {
                let proof = deserialize_proof(&account_info.data);
                println!(
                    "Balance: {} COAL\nStake: {} COAL",
                    token_balance,
                    amount_u64_to_string(proof.balance),
                )
            }
            Err(_) => {
                println!("Balance: {} COAL", token_balance);
            }
        }
    }
}
