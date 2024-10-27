use solana_sdk::signer::Signer;

use crate::{
    Miner,
    send_and_confirm::ComputeBudget,
};

impl Miner {
    pub async fn guild_initialize(&self) {
        let signer = self.signer();
        let ix = coal_guilds_api::sdk::initialize(signer.pubkey());
        let sig = self.send_and_confirm(&[ix], ComputeBudget::Fixed(500_000), false).await.unwrap();
        println!("sig: {}", sig);
    }
}