//  MAIN.rs
//    by Lut99
// 
//  Created:
//    15 Nov 2022, 09:18:40
//  Last edited:
//    23 Nov 2022, 17:05:42
//  Auto updated?
//    Yes
// 
//  Description:
//!   Entrypoint to the `branectl` executable.
// 

use std::net::SocketAddr;
use std::path::PathBuf;

use clap::{Parser, Subcommand};
use dotenvy::dotenv;
use log::{error, LevelFilter};

use brane_cfg::node::{Address, CommonPaths, CommonPorts, CommonServices};
use specifications::version::Version;

use brane_ctl::spec::{DockerClientVersion, GenerateSubcommand, StartSubcommand};
use brane_ctl::utils::resolve_config_path;
use brane_ctl::{generate, lifetime};


/***** STATICS *****/
lazy_static::lazy_static!{
    static ref API_DEFAULT_VERSION: String = format!("{}", bollard::API_DEFAULT_VERSION);
}





/***** ARGUMENTS *****/
/// Defines the toplevel arguments for the `branectl` tool.
#[derive(Debug, Parser)]
#[clap(name = "branectl", about = "The server-side Brane command-line interface.")]
struct Arguments {
    /// If given, prints `info` and `debug` prints.
    #[clap(long, help = "If given, prints additional information during execution.")]
    debug       : bool,
    /// The path to the node config file to use.
    #[clap(short, long, default_value = "./node.yml", help = "The 'node.yml' file that describes properties about the node itself (i.e., the location identifier, where to find directories, which ports to use, ...)")]
    node_config : PathBuf,

    /// The subcommand that can be run.
    #[clap(subcommand)]
    subcommand : CtlSubcommand,
}

/// Defines subcommands for the `branectl` tool.
#[derive(Debug, Subcommand)]
enum CtlSubcommand {
    #[clap(name = "generate", about = "Generates a new 'node.yml' file at the location indicated by --node-config.")]
    Generate {
        /// Defines any proxy node to proxy control messages through.
        #[clap(long, help = "If given, reroutes all control network traffic for this node through the given proxy.")]
        proxy : Option<Address>,

        /// Custom config path.
        #[clap(short='C', long, default_value = "./config", help = "A common ancestor for --infra-path, --secrets-path and --certs-path. See their descriptions for more info.")]
        config_path   : PathBuf,
        /// Custom certificates path.
        #[clap(short, long, default_value = "$CONFIG/certs", help = "The location of the certificate directory. Use '$CONFIG' to reference the value given by --config-path.")]
        certs_path    : PathBuf,
        /// Custom packages path.
        #[clap(short, long, default_value = "./packages", help = "The location of the package directory.")]
        packages_path : PathBuf,

        /// The address on which to launch the proxy service.
        #[clap(long, default_value = "0.0.0.0:50050", help = "The address on which the proxy service is hosted. Note that this is not picked up by Docker, so only docker services will be able to find it no matter what.")]
        prx_addr : SocketAddr,

        /// The address on which the API srevice is locally available.
        #[clap(long, default_value = "brane-prx:50051", help = "The address on which the proxy service is discoverable to other *local* services.")]
        prx_svc  : Address,

        /// Defines the possible nodes to generate a new node.yml file for.
        #[clap(subcommand)]
        kind : GenerateSubcommand,
    },

    #[clap(subcommand)]
    Certs(CertSubcommand),

    #[clap(subcommand)]
    Packages(PackageSubcommand),

    #[clap(subcommand)]
    Data(DataSubcommand),

    #[clap(name = "start", about = "Starts the local node by loading and then launching (already compiled) image files.")]
    Start{
        #[clap(short = 'S', long, default_value = "/var/run/docker.sock", help = "The path of the Docker socket to connect to.")]
        docker_socket  : PathBuf,
        #[clap(short = 'V', long, default_value = API_DEFAULT_VERSION.as_str(), help = "The version of the Docker client API that we use to connect to the engine.")]
        docker_version : DockerClientVersion,
        /// The docker-compose file that we start.
        #[clap(short, long, default_value = "docker-compose-$NODE.yml", help = "The docker-compose.yml file that defines the services to start. You can use '$NODE' to match either 'central' or 'worker', depending how we started.")]
        file           : PathBuf,

        /// The specific Brane version to start.
        #[clap(short, long, default_value = env!("CARGO_PKG_VERSION"), help = "The Brane version to import.")]
        version : Version,

        /// Defines the possible nodes and associated flags to start.
        #[clap(subcommand)]
        kind : StartSubcommand,
    },

    #[clap(name = "stop", about = "Stops the local node if it is running.")]
    Stop {
        /// The docker-compose file that we start.
        #[clap(short, long, default_value = "docker-compose-$NODE.yml", help = "The docker-compose.yml file that defines the services to stop. You can use '$NODE' to match either 'central' or 'worker', depending how we started.")]
        file : PathBuf,
    },

    #[clap(name = "version", about = "Returns the version of this CTL tool and/or the local node.")]
    Version {
        #[clap(short, long, help = "If given, shows the architecture instead of the version when using '--ctl' or '--node'.")]
        arch : bool,
        #[clap(short, long, help = "Shows the kind of node (i.e., 'central' or 'worker') instead of the version. Only relevant when using '--node'.")]
        kind : bool,
        #[clap(short, long, help = "If given, shows the version of the CTL tool in an easy-to-be-parsed format. Note that, if given in combination with '--node', this one is always reported first.")]
        ctl  : bool,
        #[clap(short, long, help = "If given, shows the local node version in an easy-to-be-parsed format. Note that, if given in combination with '--ctl', this one is always reported second.")]
        node : bool,
    },
}

/// Defines certificate-related subcommands for the `branectl` tool.
#[derive(Debug, Subcommand)]
#[clap(name = "certs", about = "Groups commands about certificate management.")]
enum CertSubcommand {
    
}

/// Defines package-related subcommands for the `branectl` tool.
#[derive(Debug, Subcommand)]
#[clap(name = "packages", about = "Groups commands about package management.")]
enum PackageSubcommand {

}

/// Defines data- and intermediate results-related subcommands for the `branectl` tool.
#[derive(Debug, Subcommand)]
#[clap(name = "data", about = "Groups commands about data and intermediate result management.")]
enum DataSubcommand {

}





/***** ENTYRPOINT *****/
#[tokio::main(flavor = "current_thread")]
async fn main() {
    // Load the .env file
    dotenv().ok();

    // Parse the arguments
    let args: Arguments = Arguments::parse();

    // Initialize the logger
    let mut logger = env_logger::builder();
    logger.format_module_path(false);
    if args.debug {
        logger.filter_module("brane", LevelFilter::Debug).init();
    } else {
        logger.filter_module("brane", LevelFilter::Warn).init();

        human_panic::setup_panic!(Metadata {
            name: "Brane CTL".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            authors: env!("CARGO_PKG_AUTHORS").replace(":", ", ").into(),
            homepage: env!("CARGO_PKG_HOMEPAGE").into(),
        });
    }

    // Now match on the command
    match args.subcommand {
        CtlSubcommand::Generate{ proxy, config_path, certs_path, packages_path, prx_addr, prx_svc, kind } => {
            // Create the common structs
            let paths    : CommonPaths    = CommonPaths{ certs: resolve_config_path(certs_path, &config_path), packages: resolve_config_path(packages_path, &config_path) };
            let ports    : CommonPorts    = CommonPorts{ prx: prx_addr };
            let services : CommonServices = CommonServices{ prx: prx_svc };

            // Call the thing with them
            if let Err(err) = generate::generate(args.node_config, proxy, config_path, paths, ports, services, kind) { error!("{}", err); std::process::exit(1); }
        },

        CtlSubcommand::Certs(subcommand) => match subcommand {
            
        },

        CtlSubcommand::Packages(subcommand) => match subcommand {
            
        },

        CtlSubcommand::Data(subcommand) => match subcommand {
            
        },

        CtlSubcommand::Start{ file, docker_socket, docker_version, version, kind, } => {
            if let Err(err) = lifetime::start(file, docker_socket, docker_version, version, args.node_config, kind).await { error!("{}", err); std::process::exit(1); };
        },

        CtlSubcommand::Stop{ file } => {
            
        },

        CtlSubcommand::Version { arch: _, kind: _, ctl: _, node: _ } => {
            
        },
    }
}
