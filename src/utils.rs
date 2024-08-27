use std::{io::Read, time::Duration};

use cached::proc_macro::cached;
use coal_api::{
    consts::*,
    state::{Config, Proof, Treasury},
};
use coal_utils::AccountDeserialize;
use serde::Deserialize;

use solana_client::client_error::{ClientError, ClientErrorKind};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_program::{pubkey::Pubkey, sysvar};
use solana_sdk::{clock::Clock, hash::Hash};
use spl_associated_token_account::get_associated_token_address;
use tokio::time::sleep;

pub const BLOCKHASH_QUERY_RETRIES: usize = 5;
pub const BLOCKHASH_QUERY_DELAY: u64 = 500;

#[derive(Clone, Hash, Eq, PartialEq)]
pub enum Resource {
    Coal,
    Ore,
    Ingots,
}

pub async fn _get_treasury(client: &RpcClient) -> Treasury {
    let data = client
        .get_account_data(&TREASURY_ADDRESS)
        .await
        .expect("Failed to get treasury account");
    *Treasury::try_from_bytes(&data).expect("Failed to parse treasury account")
}

pub async fn get_config(client: &RpcClient, resource: Resource) -> Config {
    let config_address = match resource {
        Resource::Coal => &coal_api::consts::CONFIG_ADDRESS,
        Resource::Ore => &ore_api::consts::CONFIG_ADDRESS,
        Resource::Ingots => &smelter_api::consts::CONFIG_ADDRESS,
    };
    let data = client
        .get_account_data( config_address)
        .await
        .expect("Failed to get config account");

    *Config::try_from_bytes(&data).expect("Failed to parse config account")
}

pub async fn get_proof_with_authority(client: &RpcClient, authority: Pubkey, resource: Resource) -> Proof {
    let proof_address = proof_pubkey(authority, resource);
    get_proof(client, proof_address).await
}

pub async fn get_updated_proof_with_authority(
    client: &RpcClient,
    authority: Pubkey,
    lash_hash_at: i64,
    resource: Resource,
) -> Proof {
    loop {
        let proof = get_proof_with_authority(client, authority, resource.clone()).await;
        if proof.last_hash_at.gt(&lash_hash_at) {
            return proof;
        }
        std::thread::sleep(Duration::from_millis(1000));
    }
}

pub async fn get_proof(client: &RpcClient, address: Pubkey) -> Proof {
    let data = client
        .get_account_data(&address)
        .await
        .expect("Failed to get proof account");
    *Proof::try_from_bytes(&data).expect("Failed to parse proof account")
}

pub async fn get_clock(client: &RpcClient) -> Clock {
    let data = client
        .get_account_data(&sysvar::clock::ID)
        .await
        .expect("Failed to get clock account");
    bincode::deserialize::<Clock>(&data).expect("Failed to deserialize clock")
}

pub fn amount_u64_to_string(amount: u64) -> String {
    amount_u64_to_f64(amount).to_string()
}

pub fn amount_u64_to_f64(amount: u64) -> f64 {
    (amount as f64) / 10f64.powf(TOKEN_DECIMALS as f64)
}

pub fn amount_f64_to_u64(amount: f64) -> u64 {
    (amount * 10f64.powf(TOKEN_DECIMALS as f64)) as u64
}

pub fn ask_confirm(question: &str) -> bool {
    println!("{}", question);
    loop {
        let mut input = [0];
        let _ = std::io::stdin().read(&mut input);
        match input[0] as char {
            'y' | 'Y' => return true,
            'n' | 'N' => return false,
            _ => println!("y/n only please."),
        }
    }
}

pub async fn get_latest_blockhash_with_retries(
    client: &RpcClient,
) -> Result<(Hash, u64), ClientError> {
    let mut attempts = 0;

    loop {
        if let Ok((hash, slot)) = client
            .get_latest_blockhash_with_commitment(client.commitment())
            .await
        {
            return Ok((hash, slot));
        }

        // Retry
        sleep(Duration::from_millis(BLOCKHASH_QUERY_DELAY)).await;
        attempts += 1;
        if attempts >= BLOCKHASH_QUERY_RETRIES {
            return Err(ClientError {
                request: None,
                kind: ClientErrorKind::Custom(
                    "Max retries reached for latest blockhash query".into(),
                ),
            });
        }
    }
}

pub fn get_resource_from_str(resource: &Option<String>) -> Resource {
    match resource {
        Some(resource) => match resource.as_str() {
            "ore" => Resource::Ore,
            "ingot" => Resource::Ingots,
            "coal" => Resource::Coal,
            _ => {
                println!("Error: Invalid resource type specified.");
                std::process::exit(1);
            },
        }
        None => Resource::Coal,
    }
}

pub fn get_resource_name(resource: &Resource) -> String {
    match resource {
        Resource::Coal => "COAL".to_string(),
        Resource::Ingots => "INGOTS".to_string(),
        Resource::Ore => "ORE".to_string(),
    }
}

pub fn get_resource_mint(resource: &Resource) -> Pubkey {
    match resource {
        Resource::Coal => coal_api::consts::MINT_ADDRESS,
        Resource::Ingots => smelter_api::consts::MINT_ADDRESS,
        Resource::Ore => ore_api::consts::MINT_ADDRESS,
    }
}

#[cached]
pub fn proof_pubkey(authority: Pubkey, resource: Resource) -> Pubkey {
    let program_id = match resource {
        Resource::Coal => &coal_api::ID,
        Resource::Ore => &ore_api::ID,
        Resource::Ingots => &smelter_api::ID,
    };
    Pubkey::find_program_address(&[PROOF, authority.as_ref()], program_id).0
}

#[cached]
pub fn treasury_tokens_pubkey() -> Pubkey {
    get_associated_token_address(&TREASURY_ADDRESS, &MINT_ADDRESS)
}

#[derive(Debug, Deserialize)]
pub struct Tip {
    pub time: String,
    pub landed_tips_25th_percentile: f64,
    pub landed_tips_50th_percentile: f64,
    pub landed_tips_75th_percentile: f64,
    pub landed_tips_95th_percentile: f64,
    pub landed_tips_99th_percentile: f64,
    pub ema_landed_tips_50th_percentile: f64,
}

