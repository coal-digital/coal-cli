use std::str::FromStr;

use coal_guilds_api::{
    consts::{LP_MINT_ADDRESS, UNSTAKE_DELAY},
    state::member_pda
};
use solana_sdk::signer::Signer;

use crate::{
    Miner,
    GuildUnstakeArgs,
    send_and_confirm::ComputeBudget,
    utils::{amount_f64_to_u64, amount_u64_to_string}, guild::utils::deserialize_member,
};

impl Miner {
    pub async fn guild_unstake(&self, args: GuildUnstakeArgs) {
        let signer = self.signer();
        let member_address = member_pda(signer.pubkey()).0;

        let member_data = self.rpc_client.get_account_data(&member_address).await.unwrap();
        let member = deserialize_member(&member_data);

        // Get current timestamp
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        if member.last_stake_at.saturating_add(UNSTAKE_DELAY).gt(&current_time) {
            println!("Too early to unstake");
            return;
        }

        let lp_tokens_address = spl_associated_token_account::get_associated_token_address(&member_address, &LP_MINT_ADDRESS);
        // Get token account
        let Ok(Some(lp_tokens)) = self.rpc_client.get_token_account(&lp_tokens_address).await else {
            println!("Failed to fetch token account");
            return;
        };
        
        // Parse amount
        let unstake_amount: u64 = if let Some(amount) = args.amount {
            println!("amount: {}", amount);
            amount_f64_to_u64(amount)
        } else {
            u64::from_str(lp_tokens.token_amount.amount.as_str()).expect("Failed to parse token balance")
        };

        println!("Unstaking: {} LP tokens", amount_u64_to_string(unstake_amount));
        let ix = coal_guilds_api::sdk::unstake(signer.pubkey(), member.guild, unstake_amount);
        self.send_and_confirm(&[ix], ComputeBudget::Fixed(500_000), false).await.ok();
    }
}