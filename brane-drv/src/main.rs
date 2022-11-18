//  MAIN.rs
//    by Lut99
// 
//  Created:
//    30 Sep 2022, 11:59:58
//  Last edited:
//    18 Nov 2022, 15:53:46
//  Auto updated?
//    Yes
// 
//  Description:
//!   Entrypoint to the `brane-drv` service.
// 

use std::net::{SocketAddr, ToSocketAddrs};
use std::path::PathBuf;
use std::sync::Arc;

use clap::Parser;
use dotenvy::dotenv;
use log::{error, LevelFilter};
use tonic::transport::Server;

use brane_cfg::InfraPath;
use brane_tsk::grpc::DriverServiceServer;
use brane_tsk::instance::InstancePlanner;

use brane_drv::handler::DriverHandler;


/***** ARGUMENTS *****/
/// Defines the arguments that may be given to the service.
#[derive(Parser)]
#[clap(version = env!("CARGO_PKG_VERSION"))]
struct Opts {
    /// Print debug info
    #[clap(short, long, action, env = "DEBUG")]
    debug : bool,

    /// Path to the infrastructure file.
    #[clap(short, long, default_value = "/config/infra.yml", help = "The path to the infra.yml file that we use to map locations to reachable services.")]
    infra_path     : PathBuf,
    /// Path to the secrets file.
    #[clap(short, long, default_value = "/config/secrets.yml", help = "The path to the secrets.yml file that aids with providing a complete picture of the infra.yml file.")]
    secrets_path   : PathBuf,
    /// Kafka brokers
    #[clap(short, long, default_value = "localhost:9092", help = "A list of Kafka brokers to connect to.", env = "BROKERS")]
    brokers        : String,
    /// Topic to send planning commands to
    #[clap(short, long = "cmd-topic", default_value = "plr-cmd", help = "The Kafka topic on which we can send commands for the planner.", env = "COMMAND_TOPIC")]
    command_topic  : String,
    /// Topic to receive planning results on.
    #[clap(short, long = "res-topic", default_value = "plr-res", help = "The Kafka topic on which we can receive the planning results.", env = "RESULT_TOPIC")]
    result_topic   : String,
    /// Consumer group id
    #[clap(short, long, default_value = "brane-drv", help = "The group ID of this service's consumer")]
    group_id       : String,

    /// Service address to service on
    #[clap(short, long, default_value = "127.0.0.1:50053", env = "ADDRESS")]
    address : String,
}





/***** ENTRY POINT *****/
#[tokio::main]
async fn main() {
    dotenv().ok();
    let opts = Opts::parse();

    // Configure logger.
    let mut logger = env_logger::builder();
    logger.format_module_path(false);
    if opts.debug {
        logger.filter_level(LevelFilter::Debug).init();
    } else {
        logger.filter_level(LevelFilter::Info).init();
    }

    // Combine the infra paths
    let infra_path: InfraPath = InfraPath::new(opts.infra_path, opts.secrets_path);

    // Create our side of the planner, and launch its event monitor
    let planner: Arc<InstancePlanner> = match InstancePlanner::new(&opts.command_topic, &opts.result_topic, &opts.brokers) {
        Ok(planner) => Arc::new(planner),
        Err(err)    => { error!("Failed to create InstancePlanner: {}", err); std::process::exit(1); },
    };
    if let Err(err) = planner.start_event_monitor(&opts.group_id).await { error!("Failed to start InstancePlanner event monitor: {}", err); std::process::exit(1); }

    // Start the DriverHandler
    let handler = DriverHandler::new(
        infra_path,
        opts.command_topic,
        planner.clone(),
    );

    // Parse the socket address(es)
    let addr: SocketAddr = match opts.address.to_socket_addrs() {
        Ok(mut addr) => match addr.next() {
            Some(addr) => addr,
            None       => { error!("Missing socket address in '{}'", opts.address); std::process::exit(1); }
        },
        Err(err) => { error!("Failed to parse '{}' as a socket address: {}", opts.address, err); std::process::exit(1); },
    };

    // Start gRPC server with callback service.
    if let Err(err) = Server::builder()
        .add_service(DriverServiceServer::new(handler))
        .serve(addr)
        .await
    {
        error!("Failed to start gRPC server: {}", err);
        std::process::exit(1);
    }
}
