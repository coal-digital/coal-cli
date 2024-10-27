use coal_guilds_api::state::{member_pda, Member};
use solana_sdk::signer::Signer;
use steel::AccountDeserialize;

use crate::{
    Miner,
    GuildLeaveArgs,
    send_and_confirm::ComputeBudget,
};

impl Miner {
    pub async fn leave_guild(&self, _args: GuildLeaveArgs) {
        let signer = self.signer();
        let member = member_pda(signer.pubkey());
        let member_data = self.rpc_client.get_account_data(&member.0).await.unwrap();
        let member = Member::try_from_bytes(&member_data).unwrap();
        let ix = coal_guilds_api::sdk::leave(signer.pubkey(), member.guild);
        let sig = self.send_and_confirm(&[ix], ComputeBudget::Fixed(500_000), false).await.unwrap();
        println!("sig: {}", sig);
    }
}