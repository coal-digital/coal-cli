use crate::{
    args::{ChopArgs, MineArgs},
    Miner,
};

impl Miner {
    pub async fn chop(&self, args: ChopArgs) {
        let mine_args = MineArgs {
            cores: args.cores,
            buffer_time: args.buffer_time,
            merged: "none".to_string(),
            resource: Some("wood".to_string()),
        };
        self.mine(mine_args).await
    }
}