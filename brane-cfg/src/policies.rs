//  POLICIES.rs
//    by Lut99
// 
//  Created:
//    01 Dec 2022, 09:20:32
//  Last edited:
//    06 Dec 2022, 11:32:16
//  Auto updated?
//    Yes
// 
//  Description:
//!   Temporary config file that is used to read simple policies until we
//!   have eFLINT
// 

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use tokio::fs as tfs;

use brane_shr::debug::EnumDebug;

pub use crate::errors::PolicyFileError as Error;


/***** LIBRARY *****/
/// Defines the toplevel policy file.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PolicyFile {
    /// The users to allow
    pub users      : Vec<UserPolicy>,
    /// The containers to allow
    pub containers : Vec<ContainerPolicy>,
}

impl PolicyFile {
    /// Constructor for the PolicyFile that reads its contents from the given YAML file.
    /// 
    /// # Arguments
    /// - `path`: The path to the policy file to load.
    /// 
    /// # Returns
    /// A new PolicyFile instance with the contents of the given file.
    /// 
    /// # Errors
    /// This function errors if we failed to read the given policy file.
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, Error> {
        let path: &Path = path.as_ref();

        // Read the file to a string
        let raw: String = match fs::read_to_string(path) {
            Ok(raw)  => raw,
            Err(err) => { return Err(Error::FileReadError { path: path.into(), err }); },
        };

        // Parse the file with serde
        match serde_yaml::from_str(&raw) {
            Ok(this) => Ok(this),
            Err(err) => Err(Error::FileParseError { path: path.into(), err }),
        }
    }

    /// Constructor for the PolicyFile that reads its contents from the given YAML file in async mode.
    /// 
    /// # Arguments
    /// - `path`: The path to the policy file to load.
    /// 
    /// # Returns
    /// A new PolicyFile instance with the contents of the given file.
    /// 
    /// # Errors
    /// This function errors if we failed to read the given policy file.
    pub async fn from_path_async(path: impl AsRef<Path>) -> Result<Self, Error> {
        let path: &Path = path.as_ref();

        // Read the file to a string
        let raw: String = match tfs::read_to_string(path).await {
            Ok(raw)  => raw,
            Err(err) => { return Err(Error::FileReadError { path: path.into(), err }); },
        };

        // Parse the file with serde
        match serde_yaml::from_str(&raw) {
            Ok(this) => Ok(this),
            Err(err) => Err(Error::FileParseError { path: path.into(), err }),
        }
    }
}



/// Defines the possible policies for users.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", tag = "policy")]
pub enum UserPolicy {
    /// Allows everyone to do anything.
    AllowAll,
    /// Denies everyone anything.
    DenyAll,

    /// Allows this user to do anything.
    AllowUserAll {
        /// The name/ID of the user as found in their certificate
        name : String,
    },
    /// Denies this user anything.
    DenyUserAll {
        /// The name/ID of the user as found in their certificate.
        name : String,
    },

    /// Allows this user to do anything on a limited set of datasets.
    Allow {
        /// The name/ID of the user as found in their certificate.
        name : String,
        /// The datasets to allow the operations for.
        data : Vec<String>,
    },
    /// Deny this user to do thing on a limited set of datasets.
    Deny {
        /// The name/ID of the user as found on their certificate.
        name : String,
        /// The datasets for which to deny them.
        data : Vec<String>,
    },
}

impl EnumDebug for UserPolicy {
    fn fmt_name(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use UserPolicy::*;
        match self {
            AllowAll => write!(f, "AllowAll"),
            DenyAll  => write!(f, "DenyAll"),

            AllowUserAll{ .. } => write!(f, "AllowUserAll"),
            DenyUserAll{ .. }  => write!(f, "DenyUserAll"),

            Allow{ .. } => write!(f, "Allow"),
            Deny{ .. }  => write!(f, "Deny"),
        }
    }
}



/// Defines the possible policies for containers.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", tag = "policy")]
pub enum ContainerPolicy {
    /// Allow all containers.
    AllowAll,
    /// Deny all containers.
    DenyAll,

    /// Allows a specific container.
    Allow {
        /// An optional name to identify the container in the logs
        name : Option<String>,
        /// The hash of the container to allow.
        hash : String,
    },
    /// Deny a specific container.
    Deny {
        /// An optional name to identify the container in the logs
        name : Option<String>,
        /// The hash of the container to allow.
        hash : String,
    },
}

impl EnumDebug for ContainerPolicy {
    fn fmt_name(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use ContainerPolicy::*;
        match self {
            AllowAll => write!(f, "AllowAll"),
            DenyAll  => write!(f, "DenyAll"),

            Allow{ .. } => write!(f, "Allow"),
            Deny{ .. }  => write!(f, "Deny"),
        }
    }
}
