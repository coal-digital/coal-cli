use std::str::FromStr;

use coal_guilds_api;
use solana_sdk::signer::Signer;
use solana_program::pubkey::Pubkey;

use crate::{
    Miner,
    GuildInviteArgs,
    send_and_confirm::ComputeBudget,
};

impl Miner {
    pub async fn guild_invite(&self, args: GuildInviteArgs) {
        let signer = self.signer();
        let member = Pubkey::from_str(&args.member).unwrap();
        let ix = coal_guilds_api::sdk::invite(signer.pubkey(), member);
        let sig = self.send_and_confirm(&[ix], ComputeBudget::Fixed(500_000), false).await.unwrap();
        println!("sig: {}", sig);
    }
}