//  MOD.rs
//    by Lut99
// 
//  Created:
//    25 Oct 2022, 11:34:40
//  Last edited:
//    21 Nov 2022, 15:05:18
//  Auto updated?
//    Yes
// 
//  Description:
//!   The instance use-case assumes that there are multiple agents trying
//!   to work together to make everything happen. In particular, the node
//!   that executes tasks is distinct from the one that runs the VM;
//!   moreover, there is an external planner and checker.
// 

// Declare the modules
pub mod planner;
pub mod worker;
pub mod vm;


// Pull some stuff into the crate namespace
pub use planner::InstancePlanner;
pub use vm::{InstancePlugin, InstanceVm};


// Define the states
/// The global state for the local use-case contains some indices, mostly.
#[derive(Clone, Debug)]
pub struct GlobalState {
    /// The path to the configuration for this node's environment. For us, contains the path to the infra.yml and (optional) secrets.yml files.
    pub node_config_path : std::path::PathBuf,
    /// The application identifier for this session.
    pub app_id           : crate::spec::AppId,

    /// The workflow for this session, which will be updated when a new one is received.
    pub workflow : Option<String>,

    /// The callback for the client to receive prints and other status updates on (such as the final result).
    /// 
    /// Note that this value is updated for every new connection the client makes.
    pub tx : Option<std::sync::Arc<tokio::sync::mpsc::Sender<Result<crate::grpc::ExecuteReply, tonic::Status>>>>,
}
impl brane_exe::spec::CustomGlobalState for GlobalState {}

/// The local state for the local use-case is unused.
pub type LocalState = ();
