//  MAIN.rs
//    by Lut99
// 
//  Created:
//    15 Nov 2022, 09:18:40
//  Last edited:
//    21 Nov 2022, 17:46:47
//  Auto updated?
//    Yes
// 
//  Description:
//!   Entrypoint to the `branectl` executable.
// 

use std::path::PathBuf;

use clap::{Parser, Subcommand};
use dotenvy::dotenv;
use log::{error, LevelFilter};

use brane_ctl::specs::GenerateSubcommand;
use brane_ctl::generate;


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
    Start{},

    #[clap(name = "stop", about = "Stops the local node if it is running.")]
    Stop {},

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

/// Defines the start subcommand, which basically defines the possible kinds of nodes to start.
#[derive(Debug, Subcommand)]
enum StartSubcommand {
    /// Starts a central node.
    #[clap(name = "central", about = "Starts a central node based on the values in the local node.yml file.")]
    Central {},

    /// Starts a worker node.
    #[clap(name = "worker", about = "Starts a worker node based on the values in the local node.yml file.")]
    Worker {},
}





/***** ENTYRPOINT *****/
fn main() {
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
        CtlSubcommand::Generate{ kind } => match kind {
            GenerateSubcommand::Central{ .. } => {
                if let Err(err) = generate::central(args.node_config, kind) { error!("{}", err); std::process::exit(1); }
            },

            GenerateSubcommand::Worker{ .. } => {
                if let Err(err) = generate::worker(args.node_config, kind) { error!("{}", err); std::process::exit(1); }
            },
        },

        CtlSubcommand::Certs(subcommand) => match subcommand {
            
        },

        CtlSubcommand::Packages(subcommand) => match subcommand {
            
        },

        CtlSubcommand::Data(subcommand) => match subcommand {
            
        },

        CtlSubcommand::Start{} => {

        },

        CtlSubcommand::Stop{} => {
            
        },

        CtlSubcommand::Version { arch: _, kind: _, ctl: _, node: _ } => {
            
        },
    }
}
