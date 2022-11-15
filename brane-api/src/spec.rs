//  SPEC.rs
//    by Lut99
// 
//  Created:
//    17 Oct 2022, 15:16:04
//  Last edited:
//    15 Nov 2022, 10:40:38
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
    /// Points to the directory where we can find domain certificates for validating their certificate validity.
    pub certs    : PathBuf,
    /// Points to the directory where we store the image.tar files.
    pub registry : PathBuf,
    /// Points to the Scylla database where we store package information.
    pub scylla   : Arc<Session>,
    /// Points to the infrastructure file that we use to have knowledge about the instance.
    pub infra    : InfraPath,
}
