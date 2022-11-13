//  LIB.rs
//    by Lut99
// 
//  Created:
//    04 Oct 2022, 11:08:37
//  Last edited:
//    02 Nov 2022, 11:47:32
//  Auto updated?
//    Yes
// 
//  Description:
//!   The `brane-cfg` library provides functions for reading Brane
//!   configuration files. This is mostly infrastructure-related.
// 

// Declare modules
// pub mod infrastructure;
// pub mod secrets;
// pub mod store;
pub mod errors;
pub mod spec;
pub mod certs;
pub mod creds;
pub mod secrets;
pub mod infra;

// Promote some stuff to the crate's namespace
// pub use infrastructure::Infrastructure;
// pub use secrets::Secrets;
pub use spec::{InfraLocation, InfraPath};
pub use creds::CredsFile;
pub use infra::{Error, InfraFile};
