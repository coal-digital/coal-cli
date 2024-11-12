use coal_guilds_api::state::{member_pda, config_pda, Guild, Member};
use solana_sdk::signer::Signer;
use steel::Discriminator;

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
    pub async fn list_guild(&self) {
        let config = config_pda();
        let accounts = self.rpc_client.get_program_accounts(&coal_guilds_api::id()).await.unwrap();

        let mut guilds = Vec::new();
        let mut guild_members = Vec::new();
        let mut solo_stakers = Vec::new();
        
        for (pubkey, account) in accounts {

            if account.data[0].eq(&(Guild::discriminator() as u8)) {
                let guild = deserialize_guild(&account.data);
                if guild.total_stake.gt(&0) {
                    guilds.push((pubkey, guild));
                }
            } else if account.data[0].eq(&(Member::discriminator() as u8)) {
                let member = deserialize_member(&account.data);
                if member.guild.eq(&solana_program::system_program::id()) && member.total_stake.gt(&0) {
                    solo_stakers.push((pubkey, member));
                } else if member.total_stake.gt(&0) {
                    guild_members.push((pubkey, member));
                }
            }
        }
        println!("Guilds found: {}", guilds.len());

        for (pubkey, guild) in guilds {
            println!("{}: {}", pubkey.to_string(), guild.total_stake);
            let guild_members_in_guild: Vec<_> = guild_members.iter()
                .filter(|(_, member)| member.guild.eq(&pubkey))
                .collect();
            
            println!("  Members: {}", guild_members_in_guild.len());
            for (_, member) in guild_members_in_guild {
                let percentage_of_guild_stake = (member.total_stake as f64 / guild.total_stake as f64) * 100.0;
                println!("    {}: {} ({}%)", member.authority.to_string(), member.total_stake, percentage_of_guild_stake);
            }
        }

        println!("Solo stakers found: {}", solo_stakers.len());
        for (pubkey, member) in solo_stakers {
            println!("{}: {}", pubkey.to_string(), member.total_stake);
        }

    }
}

fn calculate_multiplier(total_stake: u64, total_multiplier: u64, member_stake: u64) -> f64 {
    total_multiplier as f64 * member_stake as f64 / total_stake as f64
}