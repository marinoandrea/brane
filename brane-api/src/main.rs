//  MAIN.rs
//    by Lut99
// 
//  Created:
//    17 Oct 2022, 15:15:36
//  Last edited:
//    15 Nov 2022, 13:35:38
//  Auto updated?
//    Yes
// 
//  Description:
//!   Entrypoint to the `brane-job` service.
// 

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use clap::Parser;
use dotenv::dotenv;
use juniper::EmptySubscription;
use log::{debug, error, LevelFilter};
use scylla::{Session, SessionBuilder};
use warp::Filter;

use brane_cfg::InfraPath;

use brane_api::errors::ApiError;
use brane_api::spec::Context;
use brane_api::schema::{Mutations, Query, Schema};
use brane_api::health;
use brane_api::version;
use brane_api::infra;
use brane_api::data;
use brane_api::packages;


/***** ARGUMENTS *****/
#[derive(Parser)]
#[clap(version = env!("CARGO_PKG_VERSION"))]
struct Opts {
    /// Service address
    #[clap(short, long, default_value = "127.0.0.1:50051", env = "ADDRESS")]
    address  : String,
    /// Print debug info
    #[clap(short, long, env = "DEBUG", takes_value = false)]
    debug    : bool,
    /// The registry where we store image files.
    #[clap(short, long, default_value = "/packages", env = "REGISTRY")]
    registry : PathBuf,
    /// Scylla endpoint
    #[clap(long, default_value = "127.0.0.1:9042", env = "SCYLLA")]
    scylla   : String,

    /// The location of the certificates we use to connect to all domains.
    #[clap(short, long, default_value = "/certs", env = "CERTS")]
    certs   : PathBuf,
    /// The location of the infrastructure file
    #[clap(short, long, default_value = "/config/infra.yml", env = "INFRA")]
    infra   : PathBuf,
    /// The location of the secrets file
    #[clap(short, long, default_value = "/config/secrets.yml", env = "SECRETS")]
    secrets : PathBuf,
}





/***** ENTRYPOINT *****/
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

    // Configure Scylla.
    debug!("Connecting to scylla...");
    let scylla = match SessionBuilder::new()
        .known_node(&opts.scylla)
        .connection_timeout(Duration::from_secs(3))
        .build()
        .await
    {
        Ok(scylla)  => scylla,
        Err(reason) => { error!("{}", ApiError::ScyllaConnectError{ host: opts.scylla.clone(), err: reason }); std::process::exit(-1); }
    };
    debug!("Connected successfully.");

    if let Err(err) = ensure_db_keyspace(&scylla).await { error!("Failed to ensure database keyspace: {}", err) };
    if let Err(err) = packages::ensure_db_table(&scylla).await { error!("Failed to ensure database table: {}", err) };

    let scylla = Arc::new(scylla);
    let certs = opts.certs.clone();
    let registry = opts.registry.clone();

    // Merge the infrastructure and secrest file into one path
    let infra: InfraPath = InfraPath::new(opts.infra.clone(), opts.secrets.clone());

    // Configure Juniper.
    let context = warp::any().map(move || Context {
        certs    : certs.clone(),
        registry : registry.clone(),
        scylla   : scylla.clone(),
        infra    : infra.clone(),
    });

    let schema = Schema::new(Query {}, Mutations {}, EmptySubscription::new());
    let graphql_filter = juniper_warp::make_graphql_filter(schema, context.clone().boxed());
    let graphql = warp::path("graphql").and(graphql_filter);

    // Configure Warp.
    // Configure the data one
    let list_datasets = warp::path("data")
        .and(warp::path("info"))
        .and(warp::path::end())
        .and(warp::get())
        .and(context.clone())
        .and_then(data::list);
    let get_dataset = warp::path("data")
        .and(warp::path("info"))
        .and(warp::path::param())
        .and(warp::path::end())
        .and(warp::get())
        .and(context.clone())
        .and_then(data::get);
    let data = list_datasets.or(get_dataset);

    // Configure the packages one
    let download_package = warp::path("packages")
        .and(warp::get())
        .and(warp::path::param())
        .and(warp::path::param())
        .and(warp::path::end())
        .and(context.clone())
        .and_then(packages::download);
    let upload_package = warp::path("packages")
        .and(warp::path::end())
        .and(warp::post())
        .and(warp::filters::body::stream())
        .and(context.clone())
        .and_then(packages::upload);
    let packages = download_package.or(upload_package);

    // Configure infra
    let list_registries = warp::get()
        .and(warp::path("infra"))
        .and(warp::path("registries"))
        .and(warp::path::end())
        .and(context.clone())
        .and_then(infra::registries);
    let get_registry = warp::get()
        .and(warp::path("infra"))
        .and(warp::path("registries"))
        .and(warp::path::param())
        .and(warp::path::end())
        .and(context.clone())
        .and_then(infra::get_registry);
    let infra = get_registry.or(list_registries);
    
    // Configure the health & version
    let health = warp::path("health")
        .and(warp::path::end())
        .and_then(health::handle);
    let version = warp::path("version")
        .and(warp::path::end())
        .and_then(version::handle);

    // Construct the final routes
    let routes = data.or(packages.or(infra.or(health.or(version.or(graphql))))).with(warp::log("brane-api"));

    // Run the server
    let address: SocketAddr = match opts.address.clone().parse() {
        Ok(address) => address,
        Err(err)    => { error!("Failed to parse given address: {}", err); std::process::exit(1); },
    };
    warp::serve(routes).run(address).await;
}

///
///
///
pub async fn ensure_db_keyspace(scylla: &Session) -> Result<scylla::QueryResult, scylla::transport::errors::QueryError> {
    let query = r#"
        CREATE KEYSPACE IF NOT EXISTS brane
        WITH replication = {'class': 'SimpleStrategy', 'replication_factor' : 1};
    "#;

    scylla.query(query, &[]).await
}
