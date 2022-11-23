//  UTILS.rs
//    by Lut99
// 
//  Created:
//    23 Nov 2022, 14:15:54
//  Last edited:
//    23 Nov 2022, 14:17:24
//  Auto updated?
//    Yes
// 
//  Description:
//!   Utilities shared across the crate.
// 

use std::ffi::OsStr;
use std::path::{Component, Path, PathBuf};


/***** LIBRARY *****/
/// Function that resolves the given config path.
/// 
/// Effectively replaces '$CONFIG' by the path given.
/// 
/// # Arguments
/// - ``
/// 
/// # Returns
/// The same path as given, but now resolved.
pub fn resolve_config_path(path: PathBuf, config_path: impl AsRef<Path>) -> PathBuf {
    let config_path: &Path = config_path.as_ref();

    // Iterate over the parts to re-create it
    let mut result: PathBuf = PathBuf::new();
    for c in path.components() {
        if c == Component::Normal(OsStr::new("$CONFIG")) {
            result = result.join(config_path);
        } else {
            result = result.join(c);
        }
    }

    // Done
    result
}
