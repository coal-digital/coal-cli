use coal_guilds_api::state::member_pda;
use solana_sdk::signer::Signer;

use crate::{
    Miner,
    send_and_confirm::ComputeBudget,
};

impl Miner {
    pub async fn guild_member(&self) {
        let signer = self.signer();

        let member = member_pda(signer.pubkey());
        println!("Creating member {} for user {}", member.0.to_string(), signer.pubkey().to_string());

        self.send_and_confirm(&[coal_guilds_api::sdk::new_member(signer.pubkey())], ComputeBudget::Fixed(500_000), false).await.ok();
    }
}
