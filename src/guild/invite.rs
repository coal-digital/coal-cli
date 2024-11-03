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
        let guild_address = guild_pda(signer.pubkey()).0;
        println!("Inviting {} to guild {}", address, guild_address.to_string());
        
        let ix = coal_guilds_api::sdk::invite(signer.pubkey(), address);
        self.send_and_confirm(&[ix], ComputeBudget::Fixed(500_000), false).await.unwrap();
        println!("Share the following command with the invited member:");
        println!("coal guild join {}", guild_address.to_string());
    }
}