use std::{io::Read, time::Duration};

use cached::proc_macro::cached;
use coal_api::{
    consts::*,
    state::{Config, WoodConfig, Proof, ProofV2, Treasury, Tool},
};
use serde::Deserialize;
use coal_utils::AccountDeserialize;
use ore_api::consts::BUS_ADDRESSES as ORE_BUS_ADDRESSES;
use smelter_api::consts::BUS_ADDRESSES as SMELTER_BUS_ADDRESSES;
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
    Wood,
    Chromium,
}

pub enum ConfigType {
    General(Config),
    Wood(WoodConfig),
}

impl ConfigType {
    pub fn last_reset_at(&self) -> i64 {
        match self {
            ConfigType::General(config) => config.last_reset_at,
            ConfigType::Wood(config) => config.last_reset_at,
        }
    }

    pub fn min_difficulty(&self) -> u64 {
        match self {
            ConfigType::General(config) => config.min_difficulty,
            ConfigType::Wood(config) => config.min_difficulty,
        }
    }

    pub fn base_reward_rate(&self) -> u64 {
        match self {
            ConfigType::General(config) => config.base_reward_rate,
            ConfigType::Wood(config) => config.base_reward_rate,
        }
    }

    pub fn top_balance(&self) -> u64 {
        match self {
            ConfigType::General(config) => config.top_balance,
            ConfigType::Wood(config) => config.top_balance,
        }
    }
}

pub enum ProofType {
    Proof(Proof),
    ProofV2(ProofV2),
}

impl ProofType {
    pub fn authority(&self) -> Pubkey {
        match self {
            ProofType::Proof(proof) => proof.authority,
            ProofType::ProofV2(proof) => proof.authority,
        }
    }

    pub fn balance(&self) -> u64 {
        match self {
            ProofType::Proof(proof) => proof.balance,
            ProofType::ProofV2(proof) => proof.balance,
        }
    }

    pub fn challenge(&self) -> [u8; 32] {
        match self {
            ProofType::Proof(proof) => proof.challenge,
            ProofType::ProofV2(proof) => proof.challenge,
        }
    }

    pub fn last_hash(&self) -> [u8; 32] {
        match self {
            ProofType::Proof(proof) => proof.last_hash,
            ProofType::ProofV2(proof) => proof.last_hash,
        }
    }

    pub fn last_hash_at(&self) -> i64 {
        match self {
            ProofType::Proof(proof) => proof.last_hash_at,
            ProofType::ProofV2(proof) => proof.last_hash_at,
        }
    }

    pub fn last_stake_at(&self) -> i64 {
        match self {
            ProofType::Proof(proof) => proof.last_stake_at,
            ProofType::ProofV2(proof) => proof.last_stake_at,
        }
    }


    pub fn miner(&self) -> Pubkey {
        match self {
            ProofType::Proof(proof) => proof.miner,
            ProofType::ProofV2(proof) => proof.miner,
        }
    }

    pub fn total_hashes(&self) -> u64 {
        match self {
            ProofType::Proof(proof) => proof.total_hashes,
            ProofType::ProofV2(proof) => proof.total_hashes,
        }
    }

    pub fn total_rewards(&self) -> u64 {
        match self {
            ProofType::Proof(proof) => proof.total_rewards,
            ProofType::ProofV2(proof) => proof.total_rewards,
        }
    }

}

pub async fn _get_treasury(client: &RpcClient) -> Treasury {
    let data = client
        .get_account_data(&TREASURY_ADDRESS)
        .await
        .expect("Failed to get treasury account");
    *Treasury::try_from_bytes(&data).expect("Failed to parse treasury account")
}

pub fn get_config_pubkey(resource: &Resource) -> Pubkey {
    match resource {
        Resource::Coal => coal_api::consts::COAL_CONFIG_ADDRESS,
        Resource::Wood => coal_api::consts::WOOD_CONFIG_ADDRESS,
        Resource::Ingots => smelter_api::consts::CONFIG_ADDRESS,
        Resource::Ore => ore_api::consts::CONFIG_ADDRESS,
        Resource::Chromium => panic!("No config for resource"),
    }
}

pub fn deserialize_config(data: &[u8], resource: &Resource) -> ConfigType {
    match resource {
        Resource::Wood => ConfigType::Wood(
            *WoodConfig::try_from_bytes(&data).expect("Failed to parse wood config account")
        ),
        _ => ConfigType::General(
            *Config::try_from_bytes(&data).expect("Failed to parse config account")
        ),
    }
}

pub fn deserialize_tool(data: &[u8]) -> Tool {
    *Tool::try_from_bytes(&data).expect("Failed to parse tool account")
}

pub async fn get_config(client: &RpcClient, resource: &Resource) -> ConfigType {
    let config_address = get_config_pubkey(resource);

    let data = client
        .get_account_data( &config_address)
        .await
        .expect("Failed to get config account");

    deserialize_config(&data, resource)
}

pub async fn get_proof_with_authority(client: &RpcClient, authority: Pubkey, resource: &Resource) -> ProofType {
    let proof_address = proof_pubkey(authority, resource.clone());
    get_proof(client, &resource, proof_address).await
}

pub async fn get_updated_proof_with_authority(
    client: &RpcClient,
    resource: &Resource,
    authority: Pubkey,
    lash_hash_at: i64,
) -> ProofType {
    loop {
        let proof = get_proof_with_authority(client, authority, resource).await;
        if proof.last_hash_at().gt(&lash_hash_at) {
            return proof;
        }
        std::thread::sleep(Duration::from_millis(1000));
    }
}

pub async fn get_proof(client: &RpcClient, resource: &Resource, address: Pubkey) -> ProofType {
    let data = client
        .get_account_data(&address)
        .await
        .expect("Failed to get proof account");

    match resource {
        Resource::Wood => ProofType::ProofV2(*ProofV2::try_from_bytes(&data).expect("Failed to parse proof account")),
        _ => ProofType::Proof(*Proof::try_from_bytes(&data).expect("Failed to parse proof account")),
    }
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
            "wood" => Resource::Wood,
            "chromium" => Resource::Chromium,
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
        Resource::Wood => "WOOD".to_string(),
        Resource::Ingots => "INGOTS".to_string(),
        Resource::Ore => "ORE".to_string(),
        Resource::Chromium => "CHROMIUM".to_string(),
    }
}

pub fn get_resource_mint(resource: &Resource) -> Pubkey {
    match resource {
        Resource::Coal => coal_api::consts::COAL_MINT_ADDRESS,
        Resource::Wood => coal_api::consts::WOOD_MINT_ADDRESS,
        Resource::Ingots => smelter_api::consts::MINT_ADDRESS,
        Resource::Ore => ore_api::consts::MINT_ADDRESS,
        Resource::Chromium => coal_api::consts::CHROMIUM_MINT_ADDRESS,
    }
}

pub fn get_resource_bus_addresses(resource: &Resource) -> [Pubkey; BUS_COUNT] {
    match resource {
        Resource::Coal => COAL_BUS_ADDRESSES,
        Resource::Wood => WOOD_BUS_ADDRESSES,
        Resource::Ore => ORE_BUS_ADDRESSES,
        Resource::Ingots => SMELTER_BUS_ADDRESSES,
        Resource::Chromium => panic!("No bus addresses for resource"),
    }
}

pub fn get_tool_pubkey(authority: Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[COAL_MAIN_HAND_TOOL, authority.as_ref()], &coal_api::id()).0
}

#[cached]
pub fn proof_pubkey(authority: Pubkey, resource: Resource) -> Pubkey {
    let program_id = match resource {
        Resource::Coal => &coal_api::ID,
        Resource::Wood => &coal_api::ID,
        Resource::Ore => &ore_api::ID,
        Resource::Ingots => &smelter_api::ID,
        _ => panic!("No program id for resource"),
    };

    let seed = match resource {
        Resource::Coal => coal_api::consts::COAL_PROOF,
        Resource::Wood => coal_api::consts::WOOD_PROOF,
        Resource::Ore => ore_api::consts::PROOF,
        Resource::Ingots => smelter_api::consts::PROOF,
        _ => panic!("No seed for resource"),
    };
    Pubkey::find_program_address(&[seed, authority.as_ref()], program_id).0
}

#[cached]
pub fn treasury_tokens_pubkey() -> Pubkey {
    get_associated_token_address(&TREASURY_ADDRESS, &COAL_MINT_ADDRESS)
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
