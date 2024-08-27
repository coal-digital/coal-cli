use std::str::FromStr;

use colored::*;
use solana_program::pubkey::Pubkey;
use solana_sdk::signature::Signer;
use spl_token::amount_to_ui_amount;

use crate::{
    args::TransferArgs,
    cu_limits::CU_LIMIT_CLAIM,
    send_and_confirm::ComputeBudget,
    utils::{amount_f64_to_u64, ask_confirm, get_resource_from_str, get_resource_mint},
    Miner,
};

impl Miner {
    pub async fn transfer(&self, args: TransferArgs) {
        let signer = self.signer();
        let pubkey = signer.pubkey();
        let resource = get_resource_from_str(&args.resource);
        let mint = get_resource_mint(&resource);
        let sender_tokens =
            spl_associated_token_account::get_associated_token_address(&pubkey, &mint);
        let mut ixs = vec![];

        // Initialize recipient, if needed
        let to = Pubkey::from_str(&args.to).expect("Failed to parse recipient wallet address");
        let recipient_tokens =
            spl_associated_token_account::get_associated_token_address(&to, &mint);
        if self
            .rpc_client
            .get_token_account(&recipient_tokens)
            .await
            .is_err()
        {
            ixs.push(
                spl_associated_token_account::instruction::create_associated_token_account(
                    &signer.pubkey(),
                    &to,
                    &mint,
                    &spl_token::id(),
                ),
            );
        }

        // Parse amount to claim
        let amount = amount_f64_to_u64(args.amount);

        // Confirm user wants to claim
        if !ask_confirm(
            format!(
                "\nYou are about to transfer {}.\n\nAre you sure you want to continue? [Y/n]",
                format!(
                    "{} COAL",
                    amount_to_ui_amount(amount, coal_api::consts::TOKEN_DECIMALS)
                )
                .bold(),
            )
            .as_str(),
        ) {
            return;
        }

        // Send and confirm
        ixs.push(
            spl_token::instruction::transfer(
                &spl_token::id(),
                &sender_tokens,
                &recipient_tokens,
                &pubkey,
                &[&pubkey],
                amount,
            )
            .unwrap(),
        );
        self.send_and_confirm(&ixs, ComputeBudget::Fixed(CU_LIMIT_CLAIM), false)
            .await
            .ok();
    }
}
