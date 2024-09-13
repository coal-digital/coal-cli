use colored::Colorize;

use crate::{
    utils::{amount_u64_to_string, get_config, get_resource_from_str, get_resource_name},
    Miner, args::ConfigArgs,
};

impl Miner {
    pub async fn config(&self, args: ConfigArgs) {
        let resource = get_resource_from_str(&args.resource);
        let resource_name = get_resource_name(&resource);
        let config = get_config(&self.rpc_client, &resource).await;

        println!("{}: {}", "Last reset at".bold(), config.last_reset_at());
        println!("{}: {}", "Min difficulty".bold(), config.min_difficulty());
        println!("{}: {}", "Base reward rate".bold(), config.base_reward_rate());
        println!(
            "{}: {} {}",
            "Top stake".bold(),
            amount_u64_to_string(config.top_balance()),
            resource_name
        );
    }
}
