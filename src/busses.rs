use coal_api::{
    consts::{COAL_BUS_ADDRESSES, TOKEN_DECIMALS},
    state::Bus,
};
use coal_utils::AccountDeserialize;

use crate::Miner;

impl Miner {
    pub async fn busses(&self) {
        let client = self.rpc_client.clone();
        for address in COAL_BUS_ADDRESSES.iter() {
            let data = client.get_account_data(address).await.unwrap();
            match Bus::try_from_bytes(&data) {
                Ok(bus) => {
                    let rewards = (bus.rewards as f64) / 10f64.powf(TOKEN_DECIMALS as f64);
                    println!("Bus {}: {:} COAL", bus.id, rewards);
                }
                Err(_) => {}
            }
        }
    }
}
