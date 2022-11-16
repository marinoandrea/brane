//  NODE.rs
//    by Lut99
// 
//  Created:
//    16 Nov 2022, 16:54:43
//  Last edited:
//    16 Nov 2022, 17:41:29
//  Auto updated?
//    Yes
// 
//  Description:
//!   Defines a `node.json` file that describes the node - in particular,
//!   under which ports it is reachable, where its directories may be
//!   found, etc.
// 

use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use brane_shr::debug::EnumDebug;

pub use crate::errors::NodeConfigError as Error;


/***** LIBRARY *****/
/// Defines the possible node types.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum NodeKind {
    /// The central node, which is the user's access point and does all the orchestration.
    Central,
    /// The worker node, which lives on a hospital and does all the heavy work.
    Worker,
}

impl EnumDebug for NodeKind {
    #[inline]
    fn fmt_name(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use NodeKind::*;
        match self {
            Central => write!(f, "Central"),
            Worker  => write!(f, "Worker"),
        }
    }
}

impl FromStr for NodeKind {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "central" => Ok(Self::Central),
            "worker"  => Ok(Self::Worker),
    
            raw => Err(Error::UnknownNodeKind { raw: raw.into() }),
        }
    }
}



/// Defines the properties that are specific to a central node.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CentralConfig {
    /// Defines the paths configuration for the central node.
    pub paths : CentralPaths,
    /// Defines the ports configuration for the central node.
    pub ports : CentralPorts,
}

/// Defines where to find various paths for a central node.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CentralPaths {
    /// The path of the configuration directory.
    pub config : PathBuf,
    /// The path of the certificate directory.
    pub certs  : PathBuf,

    /// The path of the packages directory.
    pub packages : PathBuf,
}
impl Default for CentralPaths {
    #[inline]
    fn default() -> Self {
        Self {
            config : "./config".into(),
            certs  : "./config/certs".into(),

            packages : "./packages".into(),
        }
    }
}

/// Defines where to find various ports for a central node.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CentralPorts {
    /// Defines the port of the `brane-api` service.
    #[serde(alias = "registry")]
    api : u16,
    /// Defines the port of the `brane-drv` service.
    #[serde(alias = "driver")]
    drv : u16,
}
impl Default for CentralPorts {
    #[inline]
    fn default() -> Self {
        Self {
            api : 50051,
            drv : 50053,
        }
    }
}



/// Defines the properties that are specific to a worker node.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WorkerConfig {
    /// Defines the location ID of this location.
    #[serde(alias = "id")]
    pub location_id : String,

    /// Defines the paths configuration for the worker node.
    pub paths : WorkerPaths,
    /// Defines the ports configuration for the worker node.
    pub ports : WorkerPorts,
}

/// Defines where to find various paths for a worker node.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WorkerPaths {
    /// The path of the configuration directory.
    pub config : PathBuf,
    /// The path of the certificate directory.
    pub certs  : PathBuf,

    /// The path of the packages directory.
    pub packages : PathBuf,

    /// The path of the dataset directory.
    pub data         : PathBuf,
    /// The path of the results directory.
    pub results      : PathBuf,
    /// The path to the temporary dataset directory.
    pub temp_data    : PathBuf,
    /// The path of the temporary results directory.
    pub temp_results : PathBuf,
}
impl Default for WorkerPaths {
    #[inline]
    fn default() -> Self {
        Self {
            config : "./config".into(),
            certs  : "./config/certs".into(),

            packages : "./packages".into(),

            data         : "./data".into(),
            results      : "./results".into(),
            temp_data    : "/tmp/data".into(),
            temp_results : "/tmp/results".into(),
        }
    }
}

/// Defines where to find various ports for a worker node.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WorkerPorts {
    /// The port of the registry service.
    #[serde(alias = "registry")]
    pub reg : u16,
    /// The port of the job service.
    #[serde(alias = "delegate")]
    pub job : u16,
}
impl Default for WorkerPorts {
    #[inline]
    fn default() -> Self {
        Self {
            reg : 50051,
            job : 50052,
        }
    }
}



/// Defines a `node.json` file that describes the environment layout of a node (what type it is, its location ID, where to find folders/services, etc).
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind")]
pub enum NodeConfig {
     /// The central node, which is the user's access point and does all the orchestration.
     Central(CentralConfig),
     /// The worker node, which lives on a hospital and does all the heavy work.
     Worker(WorkerConfig),
}

impl NodeConfig {
    /// Constructor for the NodeConfig that reads it from the given path.
    /// 
    /// # Arguments
    /// - `path`: The path to read the NodeConfig from.
    /// 
    /// # Returns
    /// A new NodeConfig instance with the contents defined in the file.
    /// 
    /// # Errors
    /// This function errors if the given file cannot be read or has an invalid format.
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, Error> {
        let path: &Path = path.as_ref();

        // Get the raw file to parse
        let mut raw: String = String::new();
        {
            // Open the file
            let mut handle: File = match File::open(path) {
                Ok(handle) => handle,
                Err(err)   => { return Err(Error::FileOpenError { path: path.into(), err }); },
            };

            // Read the file
            if let Err(err) = handle.read_to_string(&mut raw) { return Err(Error::FileReadError { path: path.into(), err }); }
        }

        // Parse with serde
        match serde_yaml::from_str(&raw) {
            Ok(config) => Ok(config),
            Err(err)   => Err(Error::FileParseError { path: path.into(), err }),
        }
    }

    /// Writes the NodeConfig to the given path.
    /// 
    /// # Arguments
    /// - `path`: The path to write the NodeConfig to.
    /// 
    /// # Returns
    /// Nothing, but does obviously create a new file with this NodeConfig's contents.
    /// 
    /// # Errors
    /// This function errors if the given file cannot be written or we failed to serialize ourselves.
    pub fn to_path(&self, path: impl AsRef<Path>) -> Result<(), Error> {
        let path: &Path = path.as_ref();

        // Serialize the config
        let config: String = match serde_yaml::to_string(self) {
            Ok(config) => config,
            Err(err)   => { return Err(Error::ConfigSerializeError{ err }); },
        };

        // Write it
        {
            // Create the file
            let mut handle: File = match File::create(path) {
                Ok(handle) => handle,
                Err(err)   => { return Err(Error::FileCreateError { path: path.into(), err }); },
            };

            // Write the serialized config
            if let Err(err) = handle.write_all(config.as_bytes()) { return Err(Error::FileWriteError { path: path.into(), err }); }
        }

        // Done
        Ok(())
    }



    /// Returns the kind of this config.
    #[inline]
    pub fn kind(&self) -> NodeKind {
        use NodeConfig::*;
        match self {
            Central(_) => NodeKind::Central,
            Worker(_)  => NodeKind::Worker,
        }
    }

    /// Returns if this NodeKind is a `NodeKind::Central of sorts.
    #[inline]
    pub fn is_central(&self) -> bool { matches!(self, Self::Central(_)) }
    /// Returns this NodeKind as if it was a `NodeKind::Central`.
    /// 
    /// Will panic otherwise.
    #[inline]
    pub fn central(&self) -> &CentralConfig { if let Self::Central(config) = self { config } else { panic!("Cannot unwrap a {:?} as a NodeKind::Central", self.variant()); } }
    /// Consumes this NodeKind into a `NodeKind::Central`.
    /// 
    /// Will panic if it was not.
    #[inline]
    pub fn into_central(self) -> CentralConfig { if let Self::Central(config) = self { config } else { panic!("Cannot unwrap a {:?} as a NodeKind::Central", self.variant()); } }

    /// Returns if this NodeKind is a `NodeKind::Worker of sorts.
    #[inline]
    pub fn is_worker(&self) -> bool { matches!(self, Self::Worker(_)) }
    /// Returns this NodeKind as if it was a `NodeKind::Worker`.
    /// 
    /// Will panic otherwise.
    #[inline]
    pub fn worker(&self) -> &WorkerConfig { if let Self::Worker(config) = self { config } else { panic!("Cannot unwrap a {:?} as a NodeKind::Worker", self.variant()); } }
    /// Consumes this NodeKind into a `NodeKind::Worker`.
    /// 
    /// Will panic if it was not.
    #[inline]
    pub fn into_worker(self) -> WorkerConfig { if let Self::Worker(config) = self { config } else { panic!("Cannot unwrap a {:?} as a NodeKind::Worker", self.variant()); } }
}

impl EnumDebug for NodeConfig {
    fn fmt_name(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.kind().fmt_name(f)
    }
}

impl AsRef<NodeConfig> for NodeConfig {
    #[inline]
    fn as_ref(&self) -> &Self { self }
}
impl From<&NodeConfig> for NodeConfig {
    #[inline]
    fn from(value: &NodeConfig) -> Self { value.clone() }
}
impl From<&mut NodeConfig> for NodeConfig {
    #[inline]
    fn from(value: &mut NodeConfig) -> Self { value.clone() }
}
