//  MAIN.rs
//    by Lut99
// 
//  Created:
//    17 Oct 2022, 17:27:16
//  Last edited:
//    16 Nov 2022, 11:20:33
//  Auto updated?
//    Yes
// 
//  Description:
//!   Entrypoint to the `brane-plr` service.
// 

//  MAIN.rs
//    by Lut99
// 
//  Created:
//    30 Sep 2022, 16:10:59
//  Last edited:
//    17 Oct 2022, 17:27:08
//  Auto updated?
//    Yes
// 
//  Description:
//!   Entrypoint to the `brane-plr` service.
// 

use std::path::PathBuf;

use clap::Parser;
use dotenv::dotenv;
use log::LevelFilter;
use log::error;
use brane_cfg::InfraPath;
use brane_tsk::instance::InstancePlanner;


/***** ARGUMENTS *****/
#[derive(Parser)]
#[clap(version = env!("CARGO_PKG_VERSION"))]
struct Opts {
    /// Print debug info
    #[clap(short, long, action, env = "DEBUG")]
    debug : bool,

    /// Kafka brokers
    #[clap(short, long, default_value = "localhost:9092", help = "A list of Kafka brokers to connect to.", env = "BROKERS")]
    brokers       : String,
    /// Topic to receive planning commands on
    #[clap(short, long = "cmd-topic", default_value = "plr-cmd", help = "The Kafka topic on which we receive commands for the planner.", env = "COMMAND_TOPIC")]
    command_topic : String,
    /// Topic to send planning results to.
    #[clap(short, long = "res-topic", default_value = "plr-res", help = "The Kafka topic on which we send planning results.", env = "RESULT_TOPIC")]
    result_topic  : String,
    /// Consumer group id
    #[clap(short, long, default_value = "brane-drv", help = "The group ID of this service's consumer")]
    group_id      : String,

    /// The location of the `brane-api` service.,
    #[clap(short, long, default_value = "http://127.0.0.1:50051", help = "The address of this instance's `brane-api` service that we use to query information about datasets and where they live.", env = "API_ADDRESS")]
    api_address : String,
    /// Infra metadata store
    #[clap(short, long, default_value = "/config/infra.yml", help = "Infrastructure metadata store", env = "INFRA")]
    infra       : PathBuf,
    /// Secrets metadata store
    #[clap(short, long, default_value = "/config/secrets.yml", help = "Secrets file for the infrastructure metadata store", env = "SECRETS")]
    secrets     : PathBuf,
}





/***** ENTRYPOINT *****/
#[tokio::main]
async fn main() {
    // Load arguments & environment stuff
    dotenv().ok();
    let opts = Opts::parse();

    // Configure the logger.
    let mut logger = env_logger::builder();
    logger.format_module_path(false);
    if opts.debug {
        logger.filter_level(LevelFilter::Debug).init();
    } else {
        logger.filter_level(LevelFilter::Info).init();
    }

    // Collect the infra & secret into the InfraPath struct
    let infra: InfraPath = InfraPath::new(opts.infra, opts.secrets);

    // We simply start a new planner
    if let Err(err) = InstancePlanner::planner_server(opts.brokers, opts.group_id, opts.command_topic, opts.result_topic, opts.api_address, infra).await {
        error!("Failed to run InstancePlanner server: {}", err);
        std::process::exit(1);
    }
}
