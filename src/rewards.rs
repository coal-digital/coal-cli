use crate::{
    utils::{amount_u64_to_string, get_config, get_resource_from_str, get_resource_name},
    Miner, args::RewardsArgs,
};

impl Miner {
    pub async fn rewards(&self, args: RewardsArgs) {
        let resource = get_resource_from_str(&args.resource);
        let resource_name = get_resource_name(&resource);
        let config = get_config(&self.rpc_client, &resource).await;

        let mut s = format!(
            "{}: {} {}",
            config.min_difficulty(),
            amount_u64_to_string(config.base_reward_rate()),
            resource_name
        )
        .to_string();
        for i in 1..36 {
            let reward_rate = config.base_reward_rate().saturating_mul(2u64.saturating_pow(i));
            s = format!(
                "{}\n{}: {} {}",
                s,
                config.min_difficulty() as u32 + i,
                amount_u64_to_string(reward_rate),
                resource_name
            );
        }
        println!("{}", s);
    }
}
