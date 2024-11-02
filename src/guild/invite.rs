use std::str::FromStr;

use coal_guilds_api::{self, state::guild_pda};
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
        let address = Pubkey::from_str(&args.member).unwrap();
        let ix = coal_guilds_api::sdk::invite(signer.pubkey(), address);
        self.send_and_confirm(&[ix], ComputeBudget::Fixed(500_000), false).await.unwrap();

        let guild = guild_pda(signer.pubkey());
        println!("Invited {} to guild {}", args.member, guild.0.to_string());
    }
}