//  SPEC.rs
//    by Lut99
// 
//  Created:
//    06 Nov 2022, 17:05:19
//  Last edited:
//    06 Dec 2022, 11:19:22
//  Auto updated?
//    Yes
// 
//  Description:
//!   Contains (public) interfaces and structs for the `brane-reg` crate.
// 

use std::path::PathBuf;


/***** LIBRARY *****/
/// Defines the context for all of the warp paths.
#[derive(Clone, Debug)]
pub struct Context {
    /// The path to the node config file.
    pub node_config_path : PathBuf,
}
