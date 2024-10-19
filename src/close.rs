use colored::*;
use solana_sdk::signature::Signer;
use spl_token::amount_to_ui_amount;

use crate::{
    args::ClaimArgs, 
    send_and_confirm::ComputeBudget, 
    utils::{ask_confirm, get_proof_with_authority, get_resource_name, get_resource_from_str, Resource}, 
    CloseArgs,
    Miner
};

impl Miner {
    pub async fn close(&self, args: CloseArgs) {
        // Confirm proof exists
        let signer = self.signer();
        let resource = get_resource_from_str(&args.resource);
        let proof = get_proof_with_authority(&self.rpc_client, signer.pubkey(), &resource).await;

        // Confirm the user wants to close.
        if !ask_confirm(
            format!("{} You have {} {} staked in this account.\nAre you sure you want to {}close this account? [Y/n]", 
                "WARNING".yellow(),
                amount_to_ui_amount(proof.balance(), coal_api::consts::TOKEN_DECIMALS),
                get_resource_name(&resource),
                if proof.balance().gt(&0) { "claim your stake and "} else { "" }
            ).as_str()
        ) {
            return;
        }

        // Claim stake
        if proof.balance().gt(&0) {
            self.claim(ClaimArgs {
                amount: None,
                to: None,
                resource: args.resource,
            })
            .await;
        }

        // Submit close transaction
        let ix = match resource {
            Resource::Coal => coal_api::instruction::close_coal(signer.pubkey()),
            Resource::Wood => coal_api::instruction::close_wood(signer.pubkey()),
            Resource::Ore => ore_api::instruction::close(signer.pubkey()),
            Resource::Ingots => smelter_api::instruction::close(signer.pubkey()),
            _ => panic!("No close instruction for resource"),
        };
        self.send_and_confirm(&[ix], ComputeBudget::Fixed(500_000), false)
            .await
            .ok();
    }
}
