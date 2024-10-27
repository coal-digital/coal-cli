use std::str::FromStr;

use coal_guilds_api::state::Guild;
use solana_sdk::signer::Signer;
use solana_program::pubkey::Pubkey;
use steel::AccountDeserialize;


use crate::{
    Miner,
    GuildJoinArgs,
    send_and_confirm::ComputeBudget,
};

impl Miner {
    pub async fn guild_join(&self, args: GuildJoinArgs) {
        let signer = self.signer();
        let guild_address = Pubkey::from_str(&args.guild).unwrap();
        let guild_data = self.rpc_client.get_account_data(&guild_address).await.unwrap();
        let guild = Guild::try_from_bytes(&guild_data).unwrap();
        let ix = coal_guilds_api::sdk::join(signer.pubkey(), guild_address, guild.authority);
        let sig = self.send_and_confirm(&[ix], ComputeBudget::Fixed(500_000), false).await.unwrap();
        println!("sig: {}", sig);
    }
}