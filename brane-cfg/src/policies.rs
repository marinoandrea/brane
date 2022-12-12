//  POLICIES.rs
//    by Lut99
// 
//  Created:
//    01 Dec 2022, 09:20:32
//  Last edited:
//    12 Dec 2022, 13:56:08
//  Auto updated?
//    Yes
// 
//  Description:
//!   Temporary config file that is used to read simple policies until we
//!   have eFLINT
// 

use std::fs;
use std::io::Write;
use std::path::Path;

use enum_debug::EnumDebug;
use serde::{Deserialize, Serialize};
use tokio::fs as tfs;

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

    /// Writes the PolicyFile to the given writer.
    /// 
    /// # Arguments
    /// - `writer`: The writer to write the PolicyFile to.
    /// 
    /// # Returns
    /// Nothing, but does obviously populate the given writer with its own serialized contents.
    /// 
    /// # Errors
    /// This function errors if we failed to write or failed to serialize ourselves.
    pub fn to_writer(&self, writer: impl Write) -> Result<(), Error> {
        let mut writer = writer;

        // Serialize the config
        let config: String = match serde_yaml::to_string(self) {
            Ok(config) => config,
            Err(err)   => { return Err(Error::ConfigSerializeError{ err }); },
        };

        // Write it
        if let Err(err) = writer.write_all(config.as_bytes()) { return Err(Error::WriterWriteError{ err }); }

        // Done
        Ok(())
    }
}



/// Defines the possible policies for users.
#[derive(Clone, Debug, Deserialize, EnumDebug, Serialize)]
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



/// Defines the possible policies for containers.
#[derive(Clone, Debug, Deserialize, EnumDebug, Serialize)]
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
