//  SPEC.rs
//    by Lut99
// 
//  Created:
//    28 Nov 2022, 15:56:23
//  Last edited:
//    28 Nov 2022, 15:57:53
//  Auto updated?
//    Yes
// 
//  Description:
//!   Defines (public) interfaces and structs in the `brane-cli` crate.
// 

use std::path::PathBuf;
use std::sync::Arc;

use brane_exe::spec::CustomGlobalState;
use specifications::data::DataIndex;
use specifications::package::PackageIndex;


/***** LIBRARY *****/
/// The global state for the OfflineVm.
#[derive(Clone, Debug)]
pub struct GlobalState {
    /// The path to the directory where packages (and thus container images) are stored for this session.
    pub package_dir : PathBuf,
    /// The path to the directory where datasets (where we wanna copy results) are stored for this session.
    pub dataset_dir : PathBuf,
    /// The path to the directory where intermediate results will be stored for this session.
    pub results_dir : PathBuf,

    /// The package index that contains info about each package.
    pub pindex : Arc<PackageIndex>,
    /// The data index that contains info about each package.
    pub dindex : Arc<DataIndex>,
}
impl CustomGlobalState for GlobalState {}

/// The local state for the OfflineVm is unused.
pub type LocalState = ();
