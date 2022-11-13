//  MOD.rs
//    by Lut99
// 
//  Created:
//    24 Oct 2022, 15:31:57
//  Last edited:
//    26 Oct 2022, 16:50:49
//  Auto updated?
//    Yes
// 
//  Description:
//!   The offline module implements the simplest use-case, where the VM,
//!   planner and the jobs are executed on the same node using Docker.
// 

// Declare submodules
pub mod planner;
pub mod vm;


// Pull some stuff into the crate namespace
pub use planner::OfflinePlanner;
pub use vm::{OfflinePlugin, OfflineVm};


// Define the states
/// The global state for the local use-case contains some indices, mostly.
#[derive(Clone, Debug)]
pub struct GlobalState {
    /// The path to the directory where packages (and thus container images) are stored for this session.
    pub package_dir : std::path::PathBuf,
    /// The path to the directory where datasets (where we wanna copy results) are stored for this session.
    pub dataset_dir : std::path::PathBuf,
    /// The path to the directory where intermediate results will be stored for this session.
    pub results_dir : std::path::PathBuf,

    /// The package index that contains info about each package.
    pub pindex : std::sync::Arc<specifications::package::PackageIndex>,
    /// The data index that contains info about each package.
    pub dindex : std::sync::Arc<specifications::data::DataIndex>,
}
impl brane_exe::spec::CustomGlobalState for GlobalState {}

/// The local state for the local use-case is unused.
pub type LocalState = ();
