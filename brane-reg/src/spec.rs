//  SPEC.rs
//    by Lut99
// 
//  Created:
//    06 Nov 2022, 17:05:19
//  Last edited:
//    06 Nov 2022, 17:09:04
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
    /// The path to the folder with all datasets
    pub data_path    : PathBuf,
    /// The path to the folder with all intermediate results
    pub results_path : PathBuf,
}
