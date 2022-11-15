//  MAIN.rs
//    by Lut99
// 
//  Created:
//    15 Nov 2022, 09:18:40
//  Last edited:
//    15 Nov 2022, 10:35:49
//  Auto updated?
//    Yes
// 
//  Description:
//!   Entrypoint to the `branectl` executable.
// 

use std::path::PathBuf;

use clap::{Parser, Subcommand};


/***** ARGUMENTS *****/
/// Defines the toplevel arguments for the `branectl` tool.
#[derive(Debug, Parser)]
#[clap(name = "branectl", about = "The server-side Brane command-line interface.")]
struct Arguments {
    /// If given, prints `info` and `debug` prints.
    #[clap(long, help = "If given, prints additional information during execution.")]
    debug       : bool,
    /// The settings.json file that we use to read some information about the local instance.
    #[clap(short, long, default_value = "./.branectl.json", help = "Determines the configuration for this tool that contains settings and information about the current node.")]
    config_path : PathBuf,

    /// The subcommand that can be run.
    #[clap(subcommand)]
    subcommand : CtlSubcommand,
}

/// Defines subcommands for the `branectl` tool.
#[derive(Debug, Subcommand)]
enum CtlSubcommand {
    #[clap(subcommand)]
    Certs(CertSubcommand),

    #[clap(subcommand)]
    Data(DataSubcommand),

    #[clap(subcommand)]
    Packages(PackageSubcommand),

    #[clap(name = "start", about = "Starts the local node by loading and then launching (already compiled) image files.")]
    Start {

    },

    #[clap(name = "stop", about = "Stops the local node if it is running.")]
    Stop {

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

/// Defines data- and intermediate results-related subcommands for the `branectl` tool.
#[derive(Debug, Subcommand)]
#[clap(name = "data", about = "Groups commands about data and intermediate result management.")]
enum DataSubcommand {

}

/// Defines package-related subcommands for the `branectl` tool.
#[derive(Debug, Subcommand)]
#[clap(name = "packages", about = "Groups commands about package management.")]
enum PackageSubcommand {

}





/***** ENTYRPOINT *****/
fn main() {
    let args: Arguments = Arguments::parse();
}
