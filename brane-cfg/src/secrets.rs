//  SECRETS.rs
//    by Lut99
// 
//  Created:
//    04 Oct 2022, 11:31:26
//  Last edited:
//    14 Nov 2022, 09:42:11
//  Auto updated?
//    Yes
// 
//  Description:
//!   Defines functions and structs to deal with secrets in the infra
//!   file.
// 

use std::collections::HashMap;
use std::fs::File;
use std::path::Path;

use log::warn;
use serde::Deserialize;

pub use crate::errors::SecretsError as Error;
use crate::spec::InfraLocation;


/***** HELPER ENUMS *****/
/// Defines an abstraction over multiple credentials.
#[derive(Clone, Debug, Deserialize)]
pub enum Secret {}

impl Secret {
    /// Returns a very friendly name for the current secret.
    #[inline]
    pub fn kind(&self) -> &'static str {
        "<TBD>"
    }
}





/***** AUXILLARY STRUCTS *****/
/// Defines the structure of the secrets file itself.
#[derive(Clone, Debug, Deserialize)]
pub struct SecretsFile {
    /// The secrets contained within the file (wow!)
    pub secrets : HashMap<String, Secret>,
}





/***** LIBRARY *****/
/// Resolves any unresolved credentials in the given list of Locations.
/// 
/// # Arguments
/// - `locs`: The map of locations to resolve.
/// - `path`: The path to the secrest file to resolve the credentials with.
/// 
/// # Returns
/// Nothing, but does update any unresolved locations in the given `locs`.
/// 
/// # Errors
/// This function may error if we could not read or parse the secrets file, or if we could not find a secret with the appropriate ID / field.
pub fn resolve_secrets(_locs: &mut HashMap<String, InfraLocation>, path: impl AsRef<Path>) -> Result<(), Error> {
    let path : &Path = path.as_ref();

    // Get the secrets file, but allow for it not being there
    let _secrets: SecretsFile = {
        // Try to open the file
        match File::open(path) {
            // Further process it as the secrets store
            Ok(handle) => match serde_yaml::from_reader(handle) {
                Ok(locs) => locs,
                Err(err) => {
                    warn!("{} (assuming empty secrets file)", Error::FileParseError{ path: path.into(), err });
                    SecretsFile {
                        secrets : HashMap::new(),
                    }
                },
            },
            Err(err) => {
                warn!("{} (assuming empty secrets file)", Error::FileOpenError { path: path.into(), err });
                SecretsFile {
                    secrets : HashMap::new(),
                }
            },  
        }
    };

    // // Now iterate over the locations to find the secrets
    // for (_name, _loc) in locs {
    //     /* TBD */
    // }

    // Done
    Ok(())
}
