//  SPEC.rs
//    by Lut99
// 
//  Created:
//    21 Nov 2022, 17:27:52
//  Last edited:
//    21 Nov 2022, 17:46:43
//  Auto updated?
//    Yes
// 
//  Description:
//!   Defines specifications and interfaces used across modules.
// 

use std::net::SocketAddr;
use std::path::PathBuf;

use clap::Subcommand;


/***** LIBRARY *****/
/// A bit awkward here, but defines the generate subcommand, which basically defines the possible kinds of nodes to generate the node.yml config file for.
#[derive(Debug, Subcommand)]
pub enum GenerateSubcommand {
    /// Starts a central node.
    #[clap(name = "central", about = "Generates a node.yml file for a central node with default values. Check TODO to find the default values used.")]
    Central {
        /// Custom config path.
        #[clap(short='C', long, help = "If given, overrides the 'infra.yml', 'secrets.yml' and credentials paths with ones pointing to the this 'config' directory.")]
        config_path  : Option<PathBuf>,
        /// Custom `infra.yml` path.
        #[clap(short, long, help = "If given, overrides the default value location of the 'infra.yml' file. Use '$CONFIG' to reference the value given by --config-path.")]
        infra_path   : Option<PathBuf>,
        /// Custom `secrets.yml` path.
        #[clap(short, long, help = "If given, overrides the default value location of the 'secrets.yml' file. Use '$CONFIG' to reference the value given by --config-path.")]
        secrets_path : Option<PathBuf>,
        /// Custom certificates path.
        #[clap(short, long, help = "If given, overrides the default value location of the certificates. Use '$CONFIG' to reference the value given by --config-path.")]
        certs_path   : Option<PathBuf>,

        /// The address on which to launch the API service.
        #[clap(short, long, help = "If given, overrides the address on which the global registry service (API services) is hosted.")]
        api_addr : Option<SocketAddr>,
        /// The address on which to launch the driver service.
        #[clap(short, long, help = "If given, overrides the address on which the driver service is hosted.")]
        drv_addr : Option<SocketAddr>,

        /// The topic for planner commands.
        #[clap(short = 'P', long, help = "If given, overrides the topic used to submit planner commands on.")]
        plr_cmd_topic : Option<String>,
        /// The topic for planner results.
        #[clap(short, long, help = "If given, overrides the topic used to emit planner results on.")]
        plr_res_topic : Option<String>,
    },

    /// Starts a worker node.
    #[clap(name = "worker", about = "Generate a node.yml file for a worker node with default values. Check TODO to find the default values used.")]
    Worker {
        /// The location ID of this node.
        #[clap(name = "LOCATION_ID", help = "The location identifier (location ID) of this node.")]
        location_id : String,
    },
}
