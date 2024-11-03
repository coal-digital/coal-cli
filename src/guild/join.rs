use std::str::FromStr;

use coal_guilds_api;
use solana_sdk::signer::Signer;
use solana_program::pubkey::Pubkey;


use crate::{
    Miner,
    GuildJoinArgs,  
    send_and_confirm::ComputeBudget, guild::utils::deserialize_guild,
};

impl Miner {
    pub async fn guild_join(&self, args: GuildJoinArgs) {
        let signer = self.signer();
        let guild_address = Pubkey::from_str(&args.guild).unwrap();
        let guild_data = self.rpc_client.get_account_data(&guild_address).await;

        if let Ok(guild_data) = guild_data {
            println!("Joining guild {}", guild_address);
            let guild = deserialize_guild(&guild_data);
            println!("Guild authority: {}", guild.authority.to_string());
            let ix = coal_guilds_api::sdk::join(signer.pubkey(), guild_address, guild.authority);
            self.send_and_confirm(&[ix], ComputeBudget::Fixed(500_000), false).await.unwrap();
        } else {
            println!("Guild not found");
        }
    }
}