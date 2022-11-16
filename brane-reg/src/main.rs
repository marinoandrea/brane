//  MAIN.rs
//    by Lut99
// 
//  Created:
//    26 Sep 2022, 15:11:44
//  Last edited:
//    16 Nov 2022, 11:20:41
//  Auto updated?
//    Yes
// 
//  Description:
//!   Entrypoint to the `brane-reg` service.
// 

use std::net::{SocketAddr, ToSocketAddrs};
use std::path::PathBuf;
use std::sync::Arc;

use clap::Parser;
use dotenv::dotenv;
use log::{error, info, LevelFilter};
use rustls::Certificate;
use warp::Filter;

use brane_reg::spec::Context;
use brane_reg::server::serve_with_auth;
use brane_reg::health;
use brane_reg::version;
use brane_reg::data;


/***** ARGUMENTS *****/
/// Defines the arguments for the `brane-reg` service.
#[derive(Parser)]
struct Args {
    #[clap(long, action, help = "If given, provides additional debug prints on the logger.", env="DEBUG")]
    debug : bool,

    #[clap(long, default_value="/certs/server.pem", help = "Defines the path to the server certificate file.", env="SERVER_CERT_PATH")]
    server_cert_path : PathBuf,
    #[clap(long, default_value="/certs/server-key.pem", help = "Defines the path to the server key file.", env="SERVER_KEY_PATH")]
    server_key_path  : PathBuf,
    #[clap(long, default_value="/certs/ca.pem", help = "Defines the path to the certificate store file with the root for all authorized client keys.", env="CA_CERT_PATH")]
    ca_cert_path     : PathBuf,
    #[clap(short, long, default_value="/data", help = "Defines the path to the data store, which is a folder with nested folders with datasets.", env="DATA_PATH")]
    data_path        : PathBuf,
    #[clap(short, long, default_value="/results", help = "Defines the path to the result store, which is a folder with nested folders with intermediate results.", env="RESULTS_PATH")]
    results_path     : PathBuf,

    #[clap(short, long, default_value="127.0.0.1:50051", help = "Defines the address (as `<hostname>:<port>`) to listen on. Use '0.0.0.0' to listen on any hostname.", env="ADDRESS")]
    address : String,
}





/***** ENTYRPOINT *****/
#[tokio::main]
async fn main() {
    // Read the env & CLI args
    dotenv().ok();
    let args = Args::parse();

    // Setup the logger according to the debug flag
    let mut logger = env_logger::builder();
    logger.format_module_path(false);
    if args.debug {
        logger.filter_level(LevelFilter::Debug).init();
    } else {
        logger.filter_level(LevelFilter::Info).init();
    }
    info!("Initializing Brane local registry service v{}...", env!("CARGO_PKG_VERSION"));



    // Put the path in a context
    let context : Arc<Context> = Arc::new(Context {
        data_path    : args.data_path,
        results_path : args.results_path,
    });
    let context = warp::any().map(move || context.clone());



    // Prepare the filters for the webserver
    let list_assets = warp::get()
        .and(warp::path("data"))
        .and(warp::path("info"))
        .and(warp::path::end())
        .and(context.clone())
        .and_then(data::list);
    let get_asset = warp::get()
        .and(warp::path("data"))
        .and(warp::path("info"))
        .and(warp::path::param())
        .and(warp::path::end())
        .and(context.clone())
        .and_then(data::get);
    let download_asset = warp::get()
        .and(warp::ext::get::<Certificate>())
        .and(warp::path("data"))
        .and(warp::path("download"))
        .and(warp::path::param())
        .and(warp::path::end())
        .and(context.clone())
        .and_then(data::download_data);
    let download_result = warp::get()
        .and(warp::ext::get::<Certificate>())
        .and(warp::path("results"))
        .and(warp::path("download"))
        .and(warp::path::param())
        .and(warp::path::end())
        .and(context.clone())
        .and_then(data::download_result);
    let version = warp::path("version")
        .and(warp::path::end())
        .and_then(version::get);
    let health = warp::path("health")
        .and(warp::path::end())
        .and_then(health::get);
    let filter = list_assets.or(get_asset).or(download_asset).or(download_result).or(version).or(health);

    // Parse the hostname + port
    let address: SocketAddr = match args.address.to_socket_addrs() {
        Ok(mut address) => match address.next() {
            Some(address) => address,
            None          => {
                error!("No socket address found in '{}'", args.address);
                std::process::exit(1);
            },
        },
        Err(err) => {
            error!("Failed to parse '{}' as a socket address: {}", args.address, err);
            std::process::exit(1);
        }
    };

    // Run it
    match serve_with_auth(args.server_cert_path, args.server_key_path, args.ca_cert_path, filter, address).await {
        Ok(_)    => {},
        Err(err) => {
            error!("{}", err);
            std::process::exit(1);
        },
    }
}
