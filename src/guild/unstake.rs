use std::str::FromStr;

use coal_guilds_api::{consts::LP_MINT_ADDRESS, state::{member_pda, Member}};
use solana_sdk::signer::Signer;
use steel::AccountDeserialize;

use crate::{
    Miner,
    GuildUnstakeArgs,
    send_and_confirm::ComputeBudget,
    utils::{amount_f64_to_u64, amount_u64_to_string},
};

impl Miner {
    pub async fn guild_unstake(&self, args: GuildUnstakeArgs) {
        let signer = self.signer();
        let member = member_pda(signer.pubkey());
        
        let lp_tokens_address = spl_associated_token_account::get_associated_token_address(&signer.pubkey(), &LP_MINT_ADDRESS);
        // Get token account
        let Ok(Some(lp_tokens)) = self.rpc_client.get_token_account(&lp_tokens_address).await else {
            println!("Failed to fetch token account");
            return;
        };
        
        // Parse amount
        let amount: u64 = if let Some(amount) = args.amount {
            amount_f64_to_u64(amount)
        } else {
            u64::from_str(lp_tokens.token_amount.amount.as_str()).expect("Failed to parse token balance")
        };

        println!("Unstaking: {} LP tokens", amount_u64_to_string(amount));
        let member_data = self.rpc_client.get_account_data(&member.0).await.unwrap();
        let member = Member::try_from_bytes(&member_data).unwrap();
        let ix = coal_guilds_api::sdk::unstake(signer.pubkey(), member.guild, amount);
        let sig = self.send_and_confirm(&[ix], ComputeBudget::Fixed(500_000), false).await.unwrap();
        println!("sig: {}", sig);
    }
}