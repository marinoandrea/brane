//  ERRORS.rs
//    by Lut99
// 
//  Created:
//    21 Nov 2022, 15:46:26
//  Last edited:
//    21 Nov 2022, 15:49:48
//  Auto updated?
//    Yes
// 
//  Description:
//!   Defines the errors that may occur in the `brane-ctl` executable.
// 

use std::error::Error;
use std::fmt::{Display, Formatter, Result as FResult};
use std::path::PathBuf;


/***** LIBRARY *****/
#[derive(Debug)]
pub enum GenerateError {
    /// Failed to create a new file.
    FileCreateError{ path: PathBuf, err: std::io::Error },
    /// Failed to write the header to the new file.
    FileHeaderWriteError{ path: PathBuf, err: std::io::Error },
    /// Failed to write the main body to the new file.
    FileBodyWriteError{ path: PathBuf, err: brane_cfg::node::Error },
}
impl Display for GenerateError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FResult {
        use GenerateError::*;
        match self {
            FileCreateError{ path, err }      => write!(f, "Failed to create new node.yml file '{}': {}", path.display(), err),
            FileHeaderWriteError{ path, err } => write!(f, "Failed to write header to node.yml file '{}': {}", path.display(), err),
            FileBodyWriteError{ err, .. }     => write!(f, "Failed to write body to node.yml file: {}", err),
        }
    }
}
impl Error for GenerateError {}
