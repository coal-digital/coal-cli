mod args;
mod balance;
mod benchmark;
mod busses;
mod chop;
mod claim;
mod close;
mod config;
mod cu_limits;
mod dynamic_fee;
mod craft;
mod mine;
mod open;
mod proof;
mod rewards;
mod send_and_confirm;
mod stake;
mod transfer;
mod utils;
mod smelt;
mod replant;
mod reprocess;
mod equip;
mod unequip;
mod inspect;
#[cfg(feature = "admin")]
mod initialize;

use std::{sync::Arc, sync::RwLock};
use futures::StreamExt;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::protocol::Message;

use args::*;
use clap::{command, Parser, Subcommand};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    signature::{read_keypair_file, Keypair},
};
use utils::Tip;

struct Miner {
    pub keypair_filepath: Option<String>,
    pub priority_fee: Option<u64>,
    pub dynamic_fee_url: Option<String>,
    pub dynamic_fee: bool,
    pub rpc_client: Arc<RpcClient>,
    pub fee_payer_filepath: Option<String>,
    pub jito_client: Arc<RpcClient>,
    pub tip: Arc<std::sync::RwLock<u64>>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    #[command(about = "Fetch an account balance")]
    Balance(BalanceArgs),

    #[command(about = "Benchmark your hashpower")]
    Benchmark(BenchmarkArgs),

    #[command(about = "Fetch the bus account balances")]
    Busses(BussesArgs),

    #[command(about = "Chop some wood")]
    Chop(ChopArgs),

    #[command(about = "Claim your mining rewards")]
    Claim(ClaimArgs),

    #[command(about = "Close your account to recover rent")]
    Close(CloseArgs),

    #[command(about = "Fetch the program config")]
    Config(ConfigArgs),

    #[command(about = "Start mining")]
    Mine(MineArgs),

    #[command(about = "Reset the mining config")]
    Replant(ReplantArgs),

    #[command(about = "Fetch a proof account by address")]
    Proof(ProofArgs),

    #[command(about = "Fetch the current reward rate for each difficulty level")]
    Rewards(RewardsArgs),

    #[command(about = "Start smelting INGOTs")]
    Smelt(SmeltArgs),

    #[command(about = "Stake to earn a rewards multiplier")]
    Stake(StakeArgs),

    #[command(about = "Send COAL to anyone, anywhere in the world.")]
    Transfer(TransferArgs),

    #[command(about = "Reprocess")]
    Reprocess(ReprocessArgs),

    // #[cfg(feature = "admin")]
    // #[command(about = "Initialize coal")]
    // Initialize(InitializeArgs),

    // #[cfg(feature = "admin")]
    // #[command(about = "Initialize wood")]
    // InitializeWood(InitializeArgs),

    #[cfg(feature = "admin")]
    #[command(about = "Initialize chromium")]
    InitializeChromium(InitializeArgs),

    // #[cfg(feature = "admin")]
    // #[command(about = "Initialize the smelter program")]
    // InitializeSmelter(InitializeArgs),

    #[cfg(feature = "admin")]
    #[command(about = "Initialize the forge")]
    InitializeForge(InitializeArgs),

    #[cfg(feature = "admin")]
    #[command(about = "Initialize the tool")]
    NewTool(NewToolArgs),

    #[cfg(feature = "admin")]
    #[command(about = "Verify the collection authority")]
    Verify(VerifyArgs),

    #[command(about = "Craft a pickaxe")]
    Craft(CraftArgs),


    #[command(about = "Equip tool")]
    Equip(EquipArgs),

    #[command(about = "Unequip tool")]
    Unequip(UnequipArgs),

    #[command(about = "Inspect tool")]
    Inspect(InspectArgs),

}

#[derive(Parser, Debug)]
#[command(about, version)]
struct Args {
    #[arg(
        long,
        value_name = "NETWORK_URL",
        help = "Network address of your RPC provider",
        global = true
    )]
    rpc: Option<String>,

    #[clap(
        global = true,
        short = 'C',
        long = "config",
        id = "PATH",
        help = "Filepath to config file."
    )]
    config_file: Option<String>,

    #[arg(
        long,
        value_name = "KEYPAIR_FILEPATH",
        help = "Filepath to signer keypair.",
        global = true
    )]
    keypair: Option<String>,

    #[arg(
        long,
        value_name = "FEE_PAYER_FILEPATH",
        help = "Filepath to transaction fee payer keypair.",
        global = true
    )]
    fee_payer: Option<String>,

    #[arg(
        long,
        value_name = "MICROLAMPORTS",
        help = "Price to pay for compute units. If dynamic fees are enabled, this value will be used as the cap.",
        default_value = "500000",
        global = true
    )]
    priority_fee: Option<u64>,

    #[arg(
        long,
        value_name = "DYNAMIC_FEE_URL",
        help = "RPC URL to use for dynamic fee estimation.",
        global = true
    )]
    dynamic_fee_url: Option<String>,

    #[arg(long, help = "Enable dynamic priority fees", global = true)]
    dynamic_fee: bool,

    #[arg(
        long,
        value_name = "JITO", 
        help = "Add jito tip to the miner. Defaults to false.",
        global = true
    )]
    jito: bool,

    #[command(subcommand)]
    command: Commands,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    // Load the config file from custom path, the default path, or use default config values
    let cli_config = if let Some(config_file) = &args.config_file {
        solana_cli_config::Config::load(config_file).unwrap_or_else(|_| {
            eprintln!("error: Could not find config file `{}`", config_file);
            std::process::exit(1);
        })
    } else if let Some(config_file) = &*solana_cli_config::CONFIG_FILE {
        solana_cli_config::Config::load(config_file).unwrap_or_default()
    } else {
        solana_cli_config::Config::default()
    };

    // Initialize miner.
    let cluster = args.rpc.unwrap_or(cli_config.json_rpc_url);
    let default_keypair = args.keypair.unwrap_or(cli_config.keypair_path.clone());
    let fee_payer_filepath = args.fee_payer.unwrap_or(default_keypair.clone());
    let rpc_client = RpcClient::new_with_commitment(cluster, CommitmentConfig::processed());
    let jito_client =
        RpcClient::new("https://mainnet.block-engine.jito.wtf/api/v1/transactions".to_string());

    let tip = Arc::new(RwLock::new(0_u64));
    let tip_clone = Arc::clone(&tip);

    if args.jito {
        let url = "ws://bundles-api-rest.jito.wtf/api/v1/bundles/tip_stream";
        let (ws_stream, _) = connect_async(url).await.unwrap();
        let (_, mut read) = ws_stream.split();

        tokio::spawn(async move {
            while let Some(message) = read.next().await {
                if let Ok(Message::Text(text)) = message {
                    if let Ok(tips) = serde_json::from_str::<Vec<Tip>>(&text) {
                        for item in tips {
                            let mut tip = tip_clone.write().unwrap();
                            *tip =
                                (item.landed_tips_50th_percentile * (10_f64).powf(9.0)) as u64;
                        }
                    }
                }
            }
        });
    }
    
    let miner = Arc::new(Miner::new(
        Arc::new(rpc_client),
        args.priority_fee,
        Some(default_keypair),
        args.dynamic_fee_url,
        args.dynamic_fee,
        Some(fee_payer_filepath),
        Arc::new(jito_client),
        tip,
    ));

    // Execute user command.
    match args.command {
        Commands::Balance(args) => {
            miner.balance(args).await;
        }
        Commands::Benchmark(args) => {
            miner.benchmark(args).await;
        }
        Commands::Busses(args) => {
            miner.busses(args).await;
        }
        Commands::Chop(args) => {
            miner.chop(args).await;
        }
        Commands::Claim(args) => {
            miner.claim(args).await;
        }
        Commands::Close(args) => {
            miner.close(args).await;
        }
        Commands::Config(args) => {
            miner.config(args).await;
        }
        Commands::Mine(args) => {
            miner.mine(args).await;
        }
        Commands::Replant(args) => {
            miner.replant(args).await;
        }
        Commands::Proof(args) => {
            miner.proof(args).await;
        }
        Commands::Rewards(args) => {
            miner.rewards(args).await;
        }
        Commands::Smelt(args) => {
            miner.smelt(args).await;
        }
        Commands::Stake(args) => {
            miner.stake(args).await;
        }
        Commands::Transfer(args) => {
            miner.transfer(args).await;
        }
        Commands::Craft(args) => {
            miner.craft(args).await;
        }
        Commands::Equip(args) => {
            miner.equip(args).await;
        }
        Commands::Unequip(args) => {
            miner.unequip(args).await;
        }
        Commands::Inspect(args) => {
            miner.inspect(args).await;
        }
        Commands::Reprocess(args) => {
            miner.reprocess(args).await;
        }
        // #[cfg(feature = "admin")]
        // Commands::Initialize(_) => {
        //     miner.initialize().await;
        // }
        // #[cfg(feature = "admin")]
        // Commands::InitializeWood(_) => {
        //     miner.initialize_wood().await;
        // }
        #[cfg(feature = "admin")]
        Commands::InitializeChromium(_) => {
            miner.initialize_chromium().await;
        }
        // #[cfg(feature = "admin")]
        // Commands::InitializeSmelter(_) => {
        //     miner.initialize_smelter().await;
        // }
       #[cfg(feature = "admin")]
        Commands::InitializeForge(_) => {
            miner.initialize_forge().await;
        }
        #[cfg(feature = "admin")]
        Commands::NewTool(_) => {
            miner.new_tool().await;
        }
        #[cfg(feature = "admin")]
        Commands::Verify(_) => {
            miner.verify().await;
        }
    }
}

impl Miner {
    pub fn new(
        rpc_client: Arc<RpcClient>,
        priority_fee: Option<u64>,
        keypair_filepath: Option<String>,
        dynamic_fee_url: Option<String>,
        dynamic_fee: bool,
        fee_payer_filepath: Option<String>,
        jito_client: Arc<RpcClient>,
        tip: Arc<std::sync::RwLock<u64>>,
    ) -> Self {
        Self {
            rpc_client,
            keypair_filepath,
            priority_fee,
            dynamic_fee_url,
            dynamic_fee,
            fee_payer_filepath,
            jito_client,
            tip,
        }
    }

    pub fn signer(&self) -> Keypair {
        match self.keypair_filepath.clone() {
            Some(filepath) => read_keypair_file(filepath.clone())
                .expect(format!("No keypair found at {}", filepath).as_str()),
            None => panic!("No keypair provided"),
        }
    }

    pub fn fee_payer(&self) -> Keypair {
        match self.fee_payer_filepath.clone() {
            Some(filepath) => read_keypair_file(filepath.clone())
                .expect(format!("No fee payer keypair found at {}", filepath).as_str()),
            None => panic!("No fee payer keypair provided"),
        }
    }
}