use std::str::FromStr;

use mpl_core::Asset;
use solana_program::pubkey::Pubkey;
use solana_sdk::signer::Signer;

use crate::{
    Miner,
    args::InspectArgs,
    utils::{get_tool_pubkey, amount_u64_to_f64, get_resource_from_str, deserialize_tool},
};

impl Miner {
    pub async fn inspect(&self, args: InspectArgs) {
        let signer = self.signer();
        let resource = get_resource_from_str(&args.resource);
        
        if args.tool.is_none() {
            let tool_address = get_tool_pubkey(signer.pubkey(), &resource);
            let tool_data = self.rpc_client.get_account_data(&tool_address).await.unwrap();
            let tool = deserialize_tool(&tool_data, &resource);

            print_tool_info(tool.asset(), tool.authority(), amount_u64_to_f64(tool.durability()), tool.multiplier());
        } else if let Some(tool) = args.tool {
            let asset_address = Pubkey::from_str(&tool).unwrap();
            let asset_data = self.rpc_client.get_account_data(&asset_address).await.unwrap();
            let asset = Asset::from_bytes(&asset_data).unwrap();
            let attributes_plugin = asset.plugin_list.attributes.unwrap();
            let durability_attr = attributes_plugin.attributes.attribute_list.iter().find(|attr| attr.key == "durability");
            let multiplier_attr = attributes_plugin.attributes.attribute_list.iter().find(|attr| attr.key == "multiplier");
            
            let durability = durability_attr.unwrap().value.parse::<f64>().unwrap();
            let multiplier = multiplier_attr.unwrap().value.parse::<u64>().unwrap();

            print_tool_info(asset_address, asset.base.owner, durability, multiplier);
        }
    }
}

fn print_tool_info(mint: Pubkey, owner: Pubkey, durability: f64, multiplier: u64) {
    println!(
        "\n\nTool Inspected: {} \nOwner: {} \nDurability: {} \nMultiplier: {}x",
        mint,
        owner,
        durability,
        1.0 + (multiplier as f64 / 100.0),
    );
}