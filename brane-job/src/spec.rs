//  SPEC.rs
//    by Lut99
// 
//  Created:
//    30 Nov 2022, 18:05:59
//  Last edited:
//    30 Nov 2022, 18:33:18
//  Auto updated?
//    Yes
// 
//  Description:
//!   Defines (public) interfaces and structs for the `brane-job` crate.
// 

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::errors::ContainerHashesError;


/***** HELPER STRUCTS *****/
/// Defines a helper struct for reading container hashes from a file.
#[derive(Clone, Debug, Deserialize, Serialize)]
struct ContainerHashPropsSerde {
    /// The name of the hash that is only used for debugging purposes mom I swear.
    name : Option<String>,
    /// The hash itself
    hash : String,
}





/***** LIBRARY *****/
/// Defines a YAML file for container hashes.
#[derive(Clone, Debug)]
pub struct ContainerHashes {
    /// The hashes, not directly read from disk. Instead, they are a vector of ContainerHash structs.
    pub hashes : HashMap<String, ContainerHashProps>,
}

impl ContainerHashes {
    /// Constructor for the ContainerHashes that reads it from disk.
    /// 
    /// # Arguments
    /// - `path`: The path to read the ContainerHashes from.
    /// 
    /// # Returns
    /// A new ContainerHashes instance with the hashes read from disk.
    /// 
    /// # Errors
    /// This function may error if we failed to read the given file or if it was not valid ContainerHashes YAML.
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, ContainerHashesError> {
        let path: &Path = path.as_ref();

        // Load the file to string
        let hashes: String = match fs::read_to_string(path) {
            Ok(hashes) => hashes,
            Err(err)   => { return Err(ContainerHashesError::ReadError { path: path.into(), err }); },  
        };

        // Parse with YAML
        let hashes: Vec<ContainerHashPropsSerde> = match serde_yaml::from_str(&hashes) {
            Ok(hashes) => hashes,
            Err(err)   => { return Err(ContainerHashesError::ParseError { path: path.into(), err }); },
        };

        // Convert to a map
        let mut map: HashMap<String, ContainerHashProps> = HashMap::with_capacity(hashes.len());
        for props in hashes {
            if let Some(hash) = map.insert(props.hash.clone(), ContainerHashProps{ name: props.name }) {
                return Err(ContainerHashesError::DuplicateHash { path: path.into(), hash: props.hash });
            }
        }

        // Done, return us
        Ok(Self {
            hashes : map,
        })
    }

    /// Writes this ContainerHashes struct to the given file as YAML.
    /// 
    /// # Arguments
    /// - `path`: The path to write the ContainerHashes to.
    /// 
    /// # Errors
    /// This function errors if we failed to serialize ourselves or if we failed to write to the given file.
    pub fn into_path(self, path: impl AsRef<Path>) -> Result<(), ContainerHashesError> {
        let path: &Path = path.as_ref();

        // Convert the map to a vector
        let mut hashes: Vec<ContainerHashPropsSerde> = Vec::with_capacity(self.hashes.len());
        for (hash, props) in self.hashes {
            hashes.push(ContainerHashPropsSerde{ hash, name: props.name });
        }

        // Serialize it
        let hashes: String = match serde_yaml::to_string(&hashes) {
            Ok(hashes) => hashes,
            Err(err)   => { return Err(ContainerHashesError::SerializeError { err }); },
        };

        // Write it
        match fs::write(path, hashes) {
            Ok(_)    => Ok(()),
            Err(err) => Err(ContainerHashesError::WriteError { path: path.into(), err }),
        }
    }



    /// Checks if the given hash is in the ContainerHashes.
    /// 
    /// # Arguments
    /// - `hash`: The hash to check for.
    /// 
    /// # Returns
    /// True if we know it, or false otherwise.
    #[inline]
    pub fn contains(&self, hash: impl AsRef<str>) -> bool { self.hashes.contains_key(hash.as_ref()) }
}



/// Defines the properties of a container hash, interestingly enough excluding the actual hash.
#[derive(Clone, Debug)]
pub struct ContainerHashProps {
    /// The name of the hash that is only used for debugging purposes mom I swear.
    pub name : Option<String>,
}

impl AsRef<ContainerHashProps> for ContainerHashProps {
    #[inline]
    fn as_ref(&self) -> &Self { self }
}
