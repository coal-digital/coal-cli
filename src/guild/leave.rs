use coal_guilds_api::state::member_pda;
use solana_sdk::signer::Signer;

use crate::{
    Miner,
    GuildLeaveArgs,
    send_and_confirm::ComputeBudget,
};

use super::utils::deserialize_member;

impl Miner {
    pub async fn leave_guild(&self, _args: GuildLeaveArgs) {
        let signer = self.signer();
        let member = member_pda(signer.pubkey());
        let member_data = self.rpc_client.get_account_data(&member.0).await.unwrap();
        let member = deserialize_member(&member_data);

        if member.guild.eq(&solana_program::system_program::id()) {
            println!("Not a member of any guild");
            return;
        }
        println!("Leaving guild {}", member.guild);
        let ix = coal_guilds_api::sdk::leave(signer.pubkey(), member.guild);
        self.send_and_confirm(&[ix], ComputeBudget::Fixed(500_000), false).await.ok();
    }
}