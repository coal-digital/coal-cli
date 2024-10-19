use std::str::FromStr;

use colored::*;
use coal_api::consts::COAL_MINT_ADDRESS;
use solana_program::pubkey::Pubkey;
use solana_sdk::signature::Signer;
use spl_token::amount_to_ui_amount;

use crate::{
    args::ClaimArgs,
    cu_limits::CU_LIMIT_CLAIM,
    send_and_confirm::ComputeBudget,
    utils::{Resource, amount_f64_to_u64, ask_confirm, get_proof_with_authority, get_resource_name, get_resource_from_str, get_resource_mint},
    Miner,
};

impl Miner {
    pub async fn claim(&self, args: ClaimArgs) {
        let signer = self.signer();
        let pubkey = signer.pubkey();
        let resource = get_resource_from_str(&args.resource);

        let proof = get_proof_with_authority(&self.rpc_client, pubkey, &resource).await;
        let mut ixs = vec![];
        let beneficiary = match args.to {
            None => self.initialize_ata(pubkey, resource.clone()).await,
            Some(to) => {
                // Create beneficiary token account, if needed
                let wallet = Pubkey::from_str(&to).expect("Failed to parse wallet address");
                let benefiary_tokens = spl_associated_token_account::get_associated_token_address(
                    &wallet,
                    &COAL_MINT_ADDRESS,
                );
                if self
                    .rpc_client
                    .get_token_account(&benefiary_tokens)
                    .await
                    .is_err()
                {
                    ixs.push(
                        spl_associated_token_account::instruction::create_associated_token_account(
                            &pubkey,
                            &wallet,
                            &coal_api::consts::COAL_MINT_ADDRESS,
                            &spl_token::id(),
                        ),
                    );
                }
                benefiary_tokens
            }
        };

        // Parse amount to claim
        let amount = if let Some(amount) = args.amount {
            amount_f64_to_u64(amount)
        } else {
            proof.balance()
        };

        // Confirm user wants to claim
        if !ask_confirm(
            format!(
                "\nYou are about to claim {}.\n\nAre you sure you want to continue? [Y/n]",
                format!(
                    "{} {}",
                    amount_to_ui_amount(amount, coal_api::consts::TOKEN_DECIMALS),
                    get_resource_name(&resource)
                )
                .bold(),
            )
            .as_str(),
        ) {
            return;
        }

        // Send and confirm
        match resource.clone() {
            Resource::Ingots => {
                ixs.push(smelter_api::instruction::claim(pubkey, beneficiary, amount));
            },
            Resource::Ore => {
                ixs.push(ore_api::instruction::claim(pubkey, beneficiary, amount));
            },
            Resource::Coal => {
                ixs.push(coal_api::instruction::claim_coal(pubkey, beneficiary, amount));
            },
            Resource::Wood => {
                ixs.push(coal_api::instruction::claim_wood(pubkey, beneficiary, amount));
            },
            _ => panic!("No claim instruction for resource"),
        }
        self.send_and_confirm(&ixs, ComputeBudget::Fixed(CU_LIMIT_CLAIM), false)
            .await
            .ok();
    }

    pub async fn initialize_ata(&self, wallet: Pubkey, resource: Resource) -> Pubkey {
        // Initialize client.
        let signer = self.signer();
        let client = self.rpc_client.clone();

        // Get mint address
        let mint_address = get_resource_mint(&resource);

        // Build instructions.
        let token_account_pubkey = spl_associated_token_account::get_associated_token_address(
            &wallet,
            &mint_address,
        );

        // Check if ata already exists
        if let Ok(Some(_ata)) = client.get_token_account(&token_account_pubkey).await {
            return token_account_pubkey;
        }
        // Sign and send transaction.
        let ix = spl_associated_token_account::instruction::create_associated_token_account(
            &signer.pubkey(),
            &signer.pubkey(),
            &mint_address,
            &spl_token::id(),
        );
        self.send_and_confirm(&[ix], ComputeBudget::Fixed(400_000), false)
            .await
            .ok();

        // Return token account address
        token_account_pubkey
    }
}