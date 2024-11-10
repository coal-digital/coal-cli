use coal_guilds_api::state::{member_pda, config_pda};
use solana_sdk::signer::Signer;

use crate::{
    Miner,
    GuildGetArgs,
    utils::amount_u64_to_string,
    guild::utils::{
        deserialize_member,
        deserialize_guild,
        deserialize_config,
    },
};

impl Miner {
    pub async fn get_guild(&self, _args: GuildGetArgs) {
        let signer = self.signer();
        let config = config_pda();
        let member = member_pda(signer.pubkey());
        let accounts = self.rpc_client.get_multiple_accounts(&[member.0, config.0]).await.unwrap();
        let config = deserialize_config(&accounts[1].as_ref().unwrap().data);
        
        println!("Total network stake: {}", amount_u64_to_string(config.total_stake));
        println!("Total staking multiplier: {}x", config.total_multiplier.to_string());

        if accounts[0].is_some() {
            let member = deserialize_member(&accounts[0].as_ref().unwrap().data);
            println!("Member stake: {}", amount_u64_to_string(member.total_stake));
            println!("Member is active: {}", if member.is_active == 1 { "Yes" } else { "No" });
            println!("Member last stake at: {}", member.last_stake_at);
            println!("Member multiplier: {}x", calculate_multiplier(config.total_stake, config.total_multiplier, member.total_stake));
            
            if member.guild.ne(&solana_program::system_program::id()) {
                let guild_data = self.rpc_client.get_account_data(&member.guild).await.unwrap();
                let guild = deserialize_guild(&guild_data);
                println!("Guild: {}", member.guild.to_string());
                println!("Guild total stake: {}", amount_u64_to_string(guild.total_stake));
                println!("Guild multiplier: {}", calculate_multiplier(config.total_stake, config.total_multiplier, guild.total_stake));
                println!("Guild last stake at: {}", guild.last_stake_at);
            } else {
                println!("Guild: None");
            }
        } else {
            println!("Member: None");
        }

    }
}

fn calculate_multiplier(total_stake: u64, total_multiplier: u64, member_stake: u64) -> f64 {
    total_multiplier as f64 * member_stake as f64 / total_stake as f64
}