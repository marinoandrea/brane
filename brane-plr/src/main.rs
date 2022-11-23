//  MAIN.rs
//    by Lut99
// 
//  Created:
//    17 Oct 2022, 17:27:16
//  Last edited:
//    22 Nov 2022, 12:02:32
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
use dotenvy::dotenv;
use log::{debug, error, info, LevelFilter};
use brane_cfg::node::NodeConfig;
use brane_tsk::instance::InstancePlanner;


/***** ARGUMENTS *****/
#[derive(Parser)]
#[clap(version = env!("CARGO_PKG_VERSION"))]
struct Opts {
    /// Print debug info
    #[clap(short, long, action, help = "If given, prints additional logging information.", env = "DEBUG")]
    debug    : bool,
    #[clap(short, long, default_value = "brane-drv", help = "The group ID of this service's consumer")]
    group_id : String,

    /// Node environment metadata store.
    #[clap(short, long, default_value = "/node.yml", help = "The path to the node environment configuration. This defines things such as where local services may be found or where to store files, as wel as this service's service address.", env = "NODE_CONFIG_PATH")]
    node_config_path : PathBuf,
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
    info!("Initializing brane-plr v{}...", env!("CARGO_PKG_VERSION"));

    // Load the config, making sure it's a central config
    debug!("Loading node.yml file '{}'...", opts.node_config_path.display());
    let node_config: NodeConfig = match NodeConfig::from_path(&opts.node_config_path) {
        Ok(config) => config,
        Err(err)   => {
            error!("Failed to load NodeConfig file: {}", err);
            std::process::exit(1);
        },
    };
    if !node_config.node.is_central() { error!("Given NodeConfig file '{}' does not have properties for a central node.", opts.node_config_path.display()); std::process::exit(1); }

    // We simply start a new planner, which takes over this function
    if let Err(err) = InstancePlanner::planner_server(opts.node_config_path, node_config, opts.group_id).await {
        error!("Failed to run InstancePlanner server: {}", err);
        std::process::exit(1);
    }

    // If the planner ever returns, so do we
}
