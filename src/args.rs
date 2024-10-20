use clap::{arg, Parser};


#[derive(Parser, Debug)]
pub struct BalanceArgs {
    #[arg(
        value_name = "ADDRESS",
        help = "The account address to fetch the balance of."
    )]
    pub address: Option<String>,
    #[arg(
        long,
        value_name = "RESOURCE",
        help = "The token to claim."
    )]
    pub resource: Option<String>,
}

#[derive(Parser, Debug)]
pub struct BenchmarkArgs {
    #[arg(
        long,
        short,
        value_name = "THREAD_COUNT",
        help = "The number of cores to use during the benchmark",
        default_value = "1"
    )]
    pub cores: u64,
}

#[derive(Parser, Debug)]
pub struct BussesArgs {
    #[arg(
        long,
        value_name = "RESOURCE",
        help = "The token to query."
    )]
    pub resource: Option<String>,
}

#[derive(Parser, Debug)]
pub struct ClaimArgs {
    #[arg(
        value_name = "AMOUNT",
        help = "The amount of rewards to claim. Defaults to max."
    )]
    pub amount: Option<f64>,

    #[arg(
        long,
        value_name = "WALLET_ADDRESS",
        help = "Wallet address to receive claimed tokens."
    )]
    pub to: Option<String>,

    #[arg(
        long,
        value_name = "RESOURCE",
        help = "The token to claim."
    )]
    pub resource: Option<String>,
}

#[derive(Parser, Debug)]
pub struct CloseArgs {
    #[arg(
        long,
        value_name = "RESOURCE",
        help = "The proof to close."
    )]
    pub resource: Option<String>,
}

#[derive(Parser, Debug)]
pub struct ConfigArgs {
    #[arg(
        long,
        value_name = "RESOURCE",
        help = "The resource to query."
    )]
    pub resource: Option<String>,
}

#[cfg(feature = "admin")]
#[derive(Parser, Debug)]
pub struct InitializeArgs {}

#[cfg(feature = "admin")]
#[derive(Parser, Debug)]
pub struct VerifyArgs {}

#[cfg(feature = "admin")]
#[derive(Parser, Debug)]
pub struct NewToolArgs {}

#[derive(Parser, Debug)]
pub struct CraftArgs {
    #[arg(
        long,
        value_name = "RESOURCE",
        help = "The tool to craft."
    )]
    pub resource: Option<String>,
}

#[derive(Parser, Debug)]
pub struct EquipArgs {
    #[arg(
        long,
        value_name = "TOOL",
        help = "The tool to inspect."
    )]
    pub tool: String,
}

#[derive(Parser, Debug)]
pub struct UnequipArgs {
    #[arg(
        long,
        value_name = "RESOURCE",
        help = "The tool to unequip."
    )]
    pub resource: Option<String>,
}

#[derive(Parser, Debug)]
pub struct InspectArgs {
    #[arg(
        long,
        value_name = "TOOL",
        help = "The tool to inspect."
    )]
    pub tool: Option<String>,
    #[arg(
        long,
        value_name = "RESOURCE",
        help = "The resource tool account to inspect."
    )]
    pub resource: Option<String>,
}

#[derive(Parser, Debug)]
pub struct MineArgs {
    #[arg(
        long,
        short,
        value_name = "CORES_COUNT",
        help = "The number of CPU cores to allocate to mining.",
        default_value = "1"
    )]
    pub cores: u64,

    #[arg(
        long,
        short,
        value_name = "SECONDS",
        help = "The number seconds before the deadline to stop mining and start submitting.",
        default_value = "5"
    )]
    pub buffer_time: u64,

    #[arg(
        long,
        short,
        value_name = "MERGED",
        help = "Whether to also mine ORE.",
        default_value = "none"
    )]
    pub merged: String,

    #[arg(
        long,
        value_name = "RESOURCE",
        help = "The resource to mine."
    )]
    pub resource: Option<String>,
}

#[derive(Parser, Debug)]
pub struct ReplantArgs {}

#[derive(Parser, Debug)]
pub struct ChopArgs {
    #[arg(
        long,
        short,
        value_name = "CORES_COUNT",
        help = "The number of CPU cores to allocate to mining.",
        default_value = "1"
    )]
    pub cores: u64,

    #[arg(
        long,
        short,
        value_name = "SECONDS",
        help = "The number seconds before the deadline to stop mining and start submitting.",
        default_value = "5"
    )]
    pub buffer_time: u64,
}

#[derive(Parser, Debug)]
pub struct ProofArgs {
    #[arg(value_name = "ADDRESS", help = "The address of the proof to fetch.")]
    pub address: Option<String>,

    #[arg(
        long,
        value_name = "RESOURCE",
        help = "The token to query."
    )]
    pub resource: Option<String>,
}

#[derive(Parser, Debug)]
pub struct RewardsArgs {
    #[arg(
        long,
        value_name = "RESOURCE",
        help = "The token to query."
    )]
    pub resource: Option<String>,
}

#[derive(Parser, Debug)]
pub struct ReprocessArgs {}

#[derive(Parser, Debug)]
pub struct SmeltArgs {
    #[arg(
        long,
        short,
        value_name = "CORES_COUNT",
        help = "The number of CPU cores to allocate to mining.",
        default_value = "1"
    )]
    pub cores: u64,

    #[arg(
        long,
        short,
        value_name = "SECONDS",
        help = "The number seconds before the deadline to stop mining and start submitting.",
        default_value = "5"
    )]
    pub buffer_time: u64,
}

#[derive(Parser, Debug)]
pub struct StakeArgs {
    #[arg(
        value_name = "AMOUNT",
        help = "The amount of COAL to stake. Defaults to max."
    )]
    pub amount: Option<f64>,

    #[arg(
        long,
        value_name = "TOKEN_ACCOUNT_ADDRESS",
        help = "Token account to send COAL from. Defaults to the associated token account."
    )]
    pub token_account: Option<String>,

    #[arg(
        long,
        value_name = "RESOURCE",
        help = "The resource to stake."
    )]
    pub resource: Option<String>,
}

#[derive(Parser, Debug)]
pub struct TransferArgs {
    #[arg(value_name = "AMOUNT", help = "The amount of COAL to transfer.")]
    pub amount: f64,

    #[arg(
        value_name = "RECIPIENT_ADDRESS",
        help = "The account address of the receipient."
    )]
    pub to: String,

    #[arg(
        long,
        value_name = "RESOURCE",
        help = "The token to transfer."
    )]
    pub resource: Option<String>,
}