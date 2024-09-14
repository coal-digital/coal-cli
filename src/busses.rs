use coal_api::{
    consts::TOKEN_DECIMALS,
    state::Bus,
};
use coal_utils::AccountDeserialize;

use crate::{Miner, args::BussesArgs, utils::{get_resource_from_str, get_resource_name, get_resource_bus_addresses}};

impl Miner {
    pub async fn busses(&self, args: BussesArgs) {
        let resource = get_resource_from_str(&args.resource);
        let resource_name = get_resource_name(&resource);
        let bus_addresses = get_resource_bus_addresses(&resource);
        
        let client = self.rpc_client.clone();
        
        for address in bus_addresses.iter() {
            let data = client.get_account_data(address).await.unwrap();

            match Bus::try_from_bytes(&data) {
                Ok(bus) => {
                    let rewards = (bus.rewards as f64) / 10f64.powf(TOKEN_DECIMALS as f64);
                    println!("Bus {}: {:} {}", bus.id, rewards, resource_name);
                }
                Err(_) => {}
            }
        }
    }
}
