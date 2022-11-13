//  SPEC.rs
//    by Lut99
// 
//  Created:
//    17 Oct 2022, 15:16:04
//  Last edited:
//    03 Nov 2022, 20:34:48
//  Auto updated?
//    Yes
// 
//  Description:
//!   Defines (public) interfaces and structs for the `brane-api` crate.
// 

use std::path::PathBuf;
use std::sync::Arc;

use scylla::Session;

use brane_cfg::InfraPath;


/***** LIBRARY *****/
/// Defines the context of all the path calls.
#[derive(Clone)]
pub struct Context {
    pub certs    : PathBuf,
    pub registry : String,
    pub scylla   : Arc<Session>,
    pub infra    : InfraPath,
}
