//  SPEC.rs
//    by Lut99
// 
//  Created:
//    21 Nov 2022, 17:27:52
//  Last edited:
//    25 Nov 2022, 16:32:45
//  Auto updated?
//    Yes
// 
//  Description:
//!   Defines specifications and interfaces used across modules.
// 

use std::net::SocketAddr;
use std::path::PathBuf;
use std::str::FromStr;

use bollard::ClientVersion;
use clap::Subcommand;

use brane_cfg::node::Address;
use brane_tsk::docker::ImageSource;

use crate::errors::DockerClientVersionParseError;


/***** AUXILLARY *****/
/// Defines a wrapper around ClientVersion that allows it to be parsed.
#[derive(Clone, Copy, Debug)]
pub struct DockerClientVersion(pub ClientVersion);

impl FromStr for DockerClientVersion {
    type Err = DockerClientVersionParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Find the dot to split on
        let dot_pos: usize = match s.find('.') {
            Some(pos) => pos,
            None      => { return Err(DockerClientVersionParseError::MissingDot{ raw: s.into() }); },
        };

        // Split it
        let major: &str = &s[..dot_pos];
        let minor: &str = &s[dot_pos + 1..];

        // Attempt to parse each of them as the appropriate integer type
        let major: usize = match usize::from_str(major) {
            Ok(major) => major,
            Err(err)  => { return Err(DockerClientVersionParseError::IllegalMajorNumber{ raw: s.into(), err }); },
        };
        let minor: usize = match usize::from_str(minor) {
            Ok(minor) => minor,
            Err(err)  => { return Err(DockerClientVersionParseError::IllegalMinorNumber{ raw: s.into(), err }); },
        };

        // Done, return the value
        Ok(DockerClientVersion(ClientVersion{ major_version: major, minor_version: minor }))
    }
}





/***** LIBRARY *****/
/// A bit awkward here, but defines the generate subcommand, which basically defines the possible kinds of nodes to generate the node.yml config file for.
#[derive(Debug, Subcommand)]
pub enum GenerateSubcommand {
    /// Starts a central node.
    #[clap(name = "central", about = "Generates a node.yml file for a central node with default values. Check TODO to find the default values used.")]
    Central {
        /// Custom `infra.yml` path.
        #[clap(short, long, default_value = "$CONFIG/infra.yml", help = "The location of the 'infra.yml' file. Use '$CONFIG' to reference the value given by --config-path.")]
        infra_path    : PathBuf,
        /// Custom `secrets.yml` path.
        #[clap(short, long, default_value = "$CONFIG/secrets.yml", help = "The location of the 'secrets.yml' file. Use '$CONFIG' to reference the value given by --config-path.")]
        secrets_path  : PathBuf,

        /// The address on which to launch the API service.
        #[clap(short, long, default_value = "0.0.0.0:50051", help = "The address on which the global registry service (API services) is hosted.")]
        api_addr : SocketAddr,
        /// The address on which to launch the driver service.
        #[clap(short, long, default_value = "0.0.0.0:50053", help = "The address on which the driver service is hosted.")]
        drv_addr : SocketAddr,

        /// The address on which the Kafka broker(s) is/are locally available.
        #[clap(short = 'B', long, default_value = "aux-kafka:9092", help = "The address on which the Kafka backend service is discoverable to other *local* services.")]
        brokers    : Vec<Address>,
        /// The address on which the Scylla database is locally available.
        #[clap(short = 'S', long, default_value = "aux-scylla:9042", help = "The address on which the Scylla database is discoverable to other *local* services.")]
        scylla_svc : Address,
        /// The address on which the API srevice is locally available.
        #[clap(short = 'A', long, default_value = "http://brane-api:50051", help = "The address on which the API service is discoverable to other *local* services.")]
        api_svc    : Address,

        /// The topic for planner commands.
        #[clap(short = 't', long, default_value = "plr-cmd", help = "The Kafka topic used to submit planner commands on.")]
        plr_cmd_topic : String,
        /// The topic for planner results.
        #[clap(short = 'T', long, default_value = "plr-res", help = "The Kafka topic used to emit planner results on.")]
        plr_res_topic : String,
    },

    /// Starts a worker node.
    #[clap(name = "worker", about = "Generate a node.yml file for a worker node with default values. Check TODO to find the default values used.")]
    Worker {
        /// The location ID of this node.
        #[clap(name = "LOCATION_ID", help = "The location identifier (location ID) of this node.")]
        location_id : String,

        /// Custom credentials file path.
        #[clap(long, default_value = "$CONFIG/creds.yml", help = "The location of the `creds.yml` file. Use `$CONFIG` to reference the value given by --config-path. ")]
        creds_path        : PathBuf,
        /// Custom data path,
        #[clap(short, long, default_value = "./data", help = "The location of the data directory.")]
        data_path         : PathBuf,
        /// Custom results path.
        #[clap(short, long, default_value = "./results", help = "The location of the results directory.")]
        results_path      : PathBuf,
        /// Custom results path.
        #[clap(short = 'D', long, default_value = "/tmp/data", help = "The location of the temporary/downloaded data directory.")]
        temp_data_path    : PathBuf,
        /// Custom results path.
        #[clap(short = 'R', long, default_value = "/tmp/results", help = "The location of the temporary/download results directory.")]
        temp_results_path : PathBuf,

        /// The address on which to launch the registry service.
        #[clap(long, default_value = "0.0.0.0:50051", help = "The address on which the local registry service is hosted.")]
        reg_addr : SocketAddr,
        /// The address on which to launch the driver service.
        #[clap(long, default_value = "0.0.0.0:50052", help = "The address on which the local delegate service is hosted.")]
        job_addr : SocketAddr,

        /// The address on which the registry service is locally available.
        #[clap(long, default_value = "https://brane-reg:50051", help = "The address on which the local registry service is discoverable to other *local* services.")]
        reg_svc : Address,
        /// The address on which the checker service is locally available.
        #[clap(long, default_value = "http://brane-chk:50053", help = "The address on which the local checker service is discoverable to other *local* services.")]
        chk_svc : Address,
    },
}



/// Defines the start subcommand, which basically defines the possible kinds of nodes to start.
#[derive(Debug, Subcommand)]
pub enum StartSubcommand {
    /// Starts a central node.
    #[clap(name = "central", about = "Starts a central node based on the values in the local node.yml file.")]
    Central {
        /// THe path (or other source) to the `aux-scylla` service.
        #[clap(short = 's', long, default_value = "Registry<scylladb/scylla:4.6.3>", help = "The image to load for the aux-scylla service. If it's a path that exists, will attempt to load that file; otherwise, assumes it's an image name in a remote registry. You can wrap your names in either `Path<...>` or `Registry<...>` if it matters.")]
        aux_scylla    : ImageSource,
        /// The path (or other source) to the `aux-kafka` service.
        #[clap(short = 'k', long, default_value = "Registry<ubuntu/kafka:3.1-22.04_beta>", help = "The image to load for the aux-kafka service. If it's a path that exists, will attempt to load that file; otherwise, assumes it's an image name in a remote registry. You can wrap your names in either `Path<...>` or `Registry<...>` if it matters.")]
        aux_kafka     : ImageSource,
        /// The path (or other source) to the `aux-zookeeper` service.
        #[clap(short = 'z', long, default_value = "Registry<ubuntu/zookeeper:3.1-22.04_beta>", help = "The image to load for the aux-zookeeper service. If it's a path that exists, will attempt to load that file; otherwise, assumes it's an image name in a remote registry. You can wrap your names in either `Path<...>` or `Registry<...>` if it matters.")]
        aux_zookeeper : ImageSource,
        /// The path (or other source) to the `aux-xenon` service.
        #[clap(short = 'm', long, default_value = "Path<./target/release/aux-xenon.tar>", help = "The image to load for the aux-xenon service. If it's a path that exists, will attempt to load that file; otherwise, assumes it's an image name in a remote registry. You can wrap your names in either `Path<...>` or `Registry<...>` if it matters.")]
        aux_xenon     : ImageSource,

        /// The path (or other source) to the `brane-prx` service.
        #[clap(short = 'P', long, default_value = "Path<./target/$MODE/brane-prx.tar>", help = "The image to load for the brane-prx service. If it's a path that exists, will attempt to load that file; otherwise, assumes it's an image name in a remote registry. You can wrap your names in either `Path<...>` or `Registry<...>` if it matters. Finally, use '$MODE' to reference the value indicated by --mode.")]
        brane_prx : ImageSource,
        /// The path (or other source) to the `brane-api` service.
        #[clap(short = 'a', long, default_value = "Path<./target/$MODE/brane-api.tar>", help = "The image to load for the brane-plr service. If it's a path that exists, will attempt to load that file; otherwise, assumes it's an image name in a remote registry. You can wrap your names in either `Path<...>` or `Registry<...>` if it matters. Finally, use '$MODE' to reference the value indicated by --mode.")]
        brane_api : ImageSource,
        /// The path (or other source) to the `brane-drv` service.
        #[clap(short = 'd', long, default_value = "Path<./target/$MODE/brane-drv.tar>", help = "The image to load for the brane-drv service. If it's a path that exists, will attempt to load that file; otherwise, assumes it's an image name in a remote registry. You can wrap your names in either `Path<...>` or `Registry<...>` if it matters. Finally, use '$MODE' to reference the value indicated by --mode.")]
        brane_drv : ImageSource,
        /// The path (or other source) to the `brane-plr` service.
        #[clap(short = 'p', long, default_value = "Path<./target/$MODE/brane-plr.tar>", help = "The image to load for the brane-plr service. If it's a path that exists, will attempt to load that file; otherwise, assumes it's an image name in a remote registry. You can wrap your names in either `Path<...>` or `Registry<...>` if it matters. Finally, use '$MODE' to reference the value indicated by --mode.")]
        brane_plr : ImageSource,
    },

    /// Starts a worker node.
    #[clap(name = "worker", about = "Starts a worker node based on the values in the local node.yml file.")]
    Worker {
        /// The path (or other source) to the `brane-prx` service.
        #[clap(short = 'P', long, default_value = "Path<./target/$MODE/brane-prx.tar>", help = "The image to load for the brane-prx service. If it's a path that exists, will attempt to load that file; otherwise, assumes it's an image name in a remote registry. You can wrap your names in either `Path<...>` or `Registry<...>` if it matters. Finally, use '$MODE' to reference the value indicated by --mode.")]
        brane_prx : ImageSource,
        /// The path (or other source) to the `brane-api` service.
        #[clap(short = 'r', long, default_value = "Path<./target/$MODE/brane-reg.tar>", help = "The image to load for the brane-reg service. If it's a path that exists, will attempt to load that file; otherwise, assumes it's an image name in a remote registry. You can wrap your names in either `Path<...>` or `Registry<...>` if it matters. Finally, use '$MODE' to reference the value indicated by --mode.")]
        brane_reg : ImageSource,
        /// The path (or other source) to the `brane-drv` service.
        #[clap(short = 'j', long, default_value = "Path<./target/$MODE/brane-job.tar>", help = "The image to load for the brane-job service. If it's a path that exists, will attempt to load that file; otherwise, assumes it's an image name in a remote registry. You can wrap your names in either `Path<...>` or `Registry<...>` if it matters. Finally, use '$MODE' to reference the value indicated by --mode.")]
        brane_job : ImageSource,
    },
}
