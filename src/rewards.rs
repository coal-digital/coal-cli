use crate::{
    utils::{Resource, amount_u64_to_string, get_config},
    Miner,
};

impl Miner {
    pub async fn rewards(&self) {
        let config = get_config(&self.rpc_client, &Resource::Coal).await;

        let mut s = format!(
            "{}: {} COAL",
            config.min_difficulty(),
            amount_u64_to_string(config.base_reward_rate())
        )
        .to_string();
        for i in 1..36 {
            let reward_rate = config.base_reward_rate().saturating_mul(2u64.saturating_pow(i));
            s = format!(
                "{}\n{}: {} COAL",
                s,
                config.min_difficulty() as u32 + i,
                amount_u64_to_string(reward_rate)
            );
        }
        println!("{}", s);
    }
}
