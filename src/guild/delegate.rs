use std::str::FromStr;

use coal_guilds_api;
use solana_sdk::signer::Signer;
use solana_program::pubkey::Pubkey;


use crate::{
    Miner,
    GuildDelegateArgs,  
    send_and_confirm::ComputeBudget,
};

impl Miner {
    pub async fn guild_delegate(&self, args: GuildDelegateArgs) {
        let signer = self.signer();
        let guild_address = Pubkey::from_str(&args.guild).unwrap();
        let guild_data = self.rpc_client.get_account_data(&guild_address).await;

        if let Ok(_) = guild_data {
            println!("Delegating stake to guild {}", guild_address);
            let ix = coal_guilds_api::sdk::delegate(signer.pubkey(), guild_address);
            self.send_and_confirm(&[ix], ComputeBudget::Fixed(500_000), false).await.unwrap();
        } else {
            println!("Guild not found");
        }
    }
}