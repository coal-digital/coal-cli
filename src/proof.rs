use std::str::FromStr;

use coal_api::consts::TOKEN_DECIMALS;
use solana_program::pubkey::Pubkey;
use solana_sdk::signature::Signer;
use spl_token::amount_to_ui_amount;

use crate::{
    args::ProofArgs,
    utils::{get_proof, proof_pubkey, get_resource_from_str, get_resource_name},
    Miner,
};

impl Miner {
    pub async fn proof(&self, args: ProofArgs) {
        let signer = self.signer();
        let resource = get_resource_from_str(&args.resource);
        let address = if let Some(address) = args.address {
            Pubkey::from_str(&address).unwrap()
        } else {
            proof_pubkey(signer.pubkey(), resource.clone())
        };
        let proof = get_proof(&self.rpc_client, &resource, address).await;
        println!("Address: {:?}", address);
        println!("Authority: {:?}", proof.authority());
        println!(
            "Balance: {:?} COAL",
            amount_to_ui_amount(proof.balance(), TOKEN_DECIMALS)
        );
        println!(
            "Last hash: {}",
            solana_sdk::hash::Hash::new_from_array(proof.last_hash()).to_string()
        );
        println!("Last hash at: {:?}", proof.last_hash_at());
        println!("Last stake at: {:?}", proof.last_stake_at());
        println!("Miner: {:?}", proof.miner());
        println!("Total hashes: {:?}", proof.total_hashes());
        println!(
            "Total rewards: {:?} {}",
            amount_to_ui_amount(proof.total_rewards(), TOKEN_DECIMALS),
            get_resource_name(&resource)
        );
    }
}
