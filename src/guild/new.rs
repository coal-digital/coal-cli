use coal_guilds_api::state::{member_pda, guild_pda};
use solana_sdk::signer::Signer;
use steel::Instruction;

use crate::{
    Miner,
    args::NewGuildArgs,
    send_and_confirm::ComputeBudget
};

impl Miner {
    pub async fn new_guild(&self, _args: NewGuildArgs) {
        let signer = self.signer();

        let member = member_pda(signer.pubkey());
        let member_data = self.rpc_client.get_account_data(&member.0).await;
        let guild = guild_pda(signer.pubkey());
        println!("creating new guild {} for authority {}", guild.0, signer.pubkey());

        let mut ixs: Vec<Instruction> = vec![];

        match member_data {
            Err(_) => {
                ixs.extend([
                    coal_guilds_api::sdk::new_member(signer.pubkey()),
                    coal_guilds_api::sdk::new_guild(signer.pubkey()),
                    coal_guilds_api::sdk::invite(signer.pubkey(), signer.pubkey()),
                    coal_guilds_api::sdk::join(signer.pubkey(), guild.0, signer.pubkey()),
                ]);
            }
            Ok(_) => {
                ixs.push(coal_guilds_api::sdk::new_guild(signer.pubkey()));
            }
        }

        let ix = coal_guilds_api::sdk::new_guild(signer.pubkey());
        self.send_and_confirm(&[ix], ComputeBudget::Fixed(500_000), false).await.ok();
    }
}