//  CREDS.rs
//    by Lut99
// 
//  Created:
//    18 Oct 2022, 13:50:11
//  Last edited:
//    18 Oct 2022, 14:11:35
//  Auto updated?
//    Yes
// 
//  Description:
//!   Defines the credentials and a file that describes them for the job
//!   service to connect with its backend.
// 

use std::fs::File;
use std::path::{Path, PathBuf};

use serde::Deserialize;

pub use crate::errors::CredsFileError as Error;


/***** AUXILLARY *****/
/// Defines the possible credentials we may encounter.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum Credentials {
    // Job node acting as a node
    /// Defines that this job node connects to the "backend" by simply spinning up the local Docker daemon.
    Local {
        /// If given, uses a non-default path to connect to the Docker daemon.
        path    : Option<PathBuf>,
        /// If given, uses a non-default client version to connect with the Docker daemon.
        version : Option<(usize, usize)>,
    },

    // Job node acting as a scheduler
    /// Defines that this job node connects to one node by use of SSH. This effectively allows the centralized Brane manager to orchestrate over nodes instead of clusters.
    Ssh {
        /// The address of the machine to connect to. Should include any ports if needed.
        address : String,
        /// The path to the key file to connect with.
        key     : PathBuf,
    },

    // Job node acting as a cluster connector
    /// Defines that this job node connects to a backend Slurm cluster.
    Slurm {
        /* TBD */
    },
    /// Defines that this job node connects to a backend Kubernetes cluster.
    Kubernetes {
        /// The address or URL of the machine to connect to. Should include the port if so.
        address : String,
        /// The path to the Kubernetes config file to connect with.
        config  : PathBuf,
    },
}





/***** LIBRARY *****/
/// Defines a file that describes how a job service may connect to its backend.
/// 
/// Note that this struct is designed to act as a "handle"; i.e., keep it only around when using it but otherwise refer to it only by path.
#[derive(Debug, Deserialize)]
pub struct CredsFile {
    /// The method of connecting
    pub method : Credentials,
}

impl CredsFile {
    /// Creates a new CredsFile by loading it from the given path.
    /// 
    /// # Arguments
    /// - `path`: The path to load the CredsFile from.
    /// 
    /// # Returns
    /// A new CredsFile instance.
    /// 
    /// # Errors
    /// This function may error if the CredsFile was missing, unreadable or incorrectly formatted.
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, Error> {
        let path: &Path = path.as_ref();

        // Open the file
        let handle: File = match File::open(path) {
            Ok(handle) => handle,
            Err(err)   => { return Err(Error::FileOpenError { path: path.into(), err }); },
        };

        // Read it with serde
        match serde_yaml::from_reader(handle) {
            Ok(result) => Ok(result),
            Err(err)   => Err(Error::FileParseError { path: path.into(), err }),
        }
    }
}
