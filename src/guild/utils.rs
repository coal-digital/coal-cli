use steel::AccountDeserialize;
use coal_guilds_api::state::{Config, Guild, Member};

pub fn deserialize_config(data: &[u8]) -> Config {
   *Config::try_from_bytes(data).unwrap()
}

pub fn deserialize_member(data: &[u8]) -> Member {
   *Member::try_from_bytes(data).unwrap()
}

pub fn deserialize_guild(data: &[u8]) -> Guild {
   *Guild::try_from_bytes(data).unwrap()
}