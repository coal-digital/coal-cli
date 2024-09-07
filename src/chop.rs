use coal_api::{consts::WOOD_EPOCH_DURATION, state::WoodConfig};

use crate::{
    args::MineArgs,
    utils::get_clock,
    Miner,
};

impl Miner {
    pub async fn chop(&self, args: MineArgs) {
        self.mine(args).await
    }

    pub async fn should_reset_wood(&self, config: WoodConfig) -> bool {
        let clock = get_clock(&self.rpc_client).await;
        config
            .last_reset_at
            .saturating_add(WOOD_EPOCH_DURATION)
            .saturating_sub(5) // Buffer
            .le(&clock.unix_timestamp)
    }

}

fn calculate_multiplier(balance: u64, top_balance: u64) -> f64 {
    1.0 + (balance as f64 / top_balance as f64).min(1.0f64)
}

fn format_duration(seconds: u32) -> String {
    let minutes = seconds / 60;
    let remaining_seconds = seconds % 60;
    format!("{:02}:{:02}", minutes, remaining_seconds)
}