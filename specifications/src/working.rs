//  WORKING.rs
//    by Lut99
// 
//  Created:
//    06 Jan 2023, 15:01:17
//  Last edited:
//    06 Jan 2023, 17:59:22
//  Auto updated?
//    Yes
// 
//  Description:
//!   Contains prost messages for interacting with the job service /
//!   worker.
// 

use std::error;
use std::fmt::{Display, Formatter, Result as FResult};

use prost::{Enumeration, Message, Oneof};

pub use JobServiceError as Error;


/***** ERRORS *****/
/// Defines the errors occuring in the JobServiceClient or JobServiceServer.
#[derive(Debug)]
pub enum JobServiceError {
    /// Failed to create an endpoint with the given address.
    EndpointError{ address: String, err: tonic::transport::Error },
    /// Failed to connect to the given address.
    ConnectError{ address: String, err: tonic::transport::Error },
}
impl Display for JobServiceError {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> FResult {
        use JobServiceError::*;
        match self {
            EndpointError{ address, err } => write!(f, "Failed to create a new Endpoint from '{}': {}", address, err),
            ConnectError{ address, err }  => write!(f, "Failed to connect to gRPC endpoint '{}': {}", address, err),
        }
    }
}
impl error::Error for JobServiceError {}





/***** AUXILLARY MESSAGES *****/
/// Auxillary enum that defines the possible kinds of datasets.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Enumeration)]
#[repr(i32)]
pub enum DataKind {
    /// A full-fledged, persistent dataset.
    Data               = 0,
    /// An intermediate result that will not outlive the scope of the workflow.
    IntermediateResult = 1,
}

/// Auxillary message for carrying a dataset with its associated name.
#[derive(Clone, Message)]
pub struct DataName {
    /// The name of the dataset.
    #[prost(tag = "1", required, string)]
    pub name : String,
    /// The kind of the dataset (i.e., Data or IntermediateResult).
    #[prost(tag = "2", required, enumeration = "DataKind")]
    pub kind : i32,
}



/// Auxillary message that implements the fields for a TransferRegistryTar PreprocessKind.
#[derive(Clone, Message)]
pub struct TransferRegistryTar {
    /// The location where the address is from.
    #[prost(tag = "1", required, string)]
    pub location : String,
    /// The address + path that, once it receives a GET-request with credentials and such, downloads the referenced dataset.
    #[prost(tag = "2", required, string)]
    pub address  : String,
}

/// Auxillary enum that defines the possible kinds of datasets.
#[derive(Clone, Oneof)]
pub enum PreprocessKind {
    /// We want to transfer it as a tar.gz from a local registry.
    #[prost(tag = "2", message)]
    TransferRegistryTar(TransferRegistryTar),
}

impl From<crate::data::PreprocessKind> for PreprocessKind {
    #[inline]
    fn from(value: crate::data::PreprocessKind) -> Self {
        match value {
            crate::data::PreprocessKind::TransferRegistryTar{ location, address } => Self::TransferRegistryTar(TransferRegistryTar{ location, address }),
        }
    }
}
impl From<&crate::data::PreprocessKind> for PreprocessKind {
    #[inline]
    fn from(value: &crate::data::PreprocessKind) -> Self { Self::from(value.clone()) }
}
impl From<&mut crate::data::PreprocessKind> for PreprocessKind {
    #[inline]
    fn from(value: &mut crate::data::PreprocessKind) -> Self { Self::from(value.clone()) }
}
impl From<PreprocessKind> for crate::data::PreprocessKind {
    #[inline]
    fn from(value: PreprocessKind) -> Self {
        match value {
            PreprocessKind::TransferRegistryTar(TransferRegistryTar{ location, address }) => crate::data::PreprocessKind::TransferRegistryTar{ location, address },
        }
    }
}
impl From<&PreprocessKind> for crate::data::PreprocessKind {
    #[inline]
    fn from(value: &PreprocessKind) -> Self { Self::from(value.clone()) }
}
impl From<&mut PreprocessKind> for crate::data::PreprocessKind {
    #[inline]
    fn from(value: &mut PreprocessKind) -> Self { Self::from(value.clone()) }
}



/// Auxillary enum that defines the possible states a task can have.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Enumeration)]
#[repr(i32)]
pub enum TaskStatus {
    // Meta events
    /// No status yet / unknown
    Unknown = 0,

    // Job events
    /// The job has been received by the job node.
    Received = 1,

    // Checker events
    /// The job has been authorized by the job's checker(s).
    Authorized          = 2,
    /// The job has been denied by the job's checker(s).
    Denied              = 3,
    /// Authorization has failed. If seen, the `value` field is also populated with the error message.
    AuthorizationFailed = 4,

    // Creation events
    /// The job container has been created.
    Created        = 5,
    /// We failed to create the job container. If seen, the `value` field is also populated with the error message.
    CreationFailed = 6,

    // Initialization events
    /// The branelet has been booted (first event it sends).
    Ready                = 7,
    /// The branelet node has been initialized; now only to spawn the job itself.
    Initialized          = 8,
    /// We failed to initialize branelet. If seen, the `value` field is also populated with the error message.
    InitializationFailed = 9,
    /// The actual subcall executeable / script has started
    Started              = 10,
    /// The subprocess executable did not want to start (calling it failed) If seen, the `value` field is also populated with the error message.
    StartingFailed       = 11,

    // Progress events
    /// Occassional message to let the user know the container is alive and running.
    Heartbeat        = 12,
    /// The package call went successfully from the branelet's side.
    Completed        = 13,
    /// The package call went wrong from the branelet's side. If seen, the `value` field is also populated with the error message.
    CompletionFailed = 14,

    // Finish events
    /// The container has exited with a zero status code and return a value. If seen, then the `value` field is populated with the JSON-encoded FullValue returned.
    Finished       = 15,
    /// The container was interrupted by the Job node
    Stopped        = 16,
    /// brane-let could not decode the output from the package call. If seen, the `value` field is also populated with the error message.
    DecodingFailed = 17,
    /// The container has exited with a non-zero status code.  If seen, the `value` field is populated with a JSON-encoded triplet of the error code, the container's stdout and the container's stderr.
    Failed         = 18,
}





/***** MESSAGES *****/
/// Request for preprocessing a given dataset.
#[derive(Clone, Message)]
pub struct PreprocessRequest {
    /// The dataset's name (and kind)
    #[prost(tag = "1", required, message)]
    pub data : DataName,
    /// The type of preprocessing that will need to happen.
    #[prost(tags = "2", oneof = "PreprocessKind")]
    pub kind : Option<PreprocessKind>,
}

/// The reply sent by the worker when the preprocessing of a dataset has been done.
#[derive(Clone, Message)]
pub struct PreprocessReply {
    /// The method of accessing this dataset from now on.
    #[prost(tag = "1", required, string)]
    pub access : String,
}



/// Request for executing a task on some domain.
#[derive(Clone, Message)]
pub struct TaskRequest {
    /// The location of the API service where information may be retrieved from.
    #[prost(tag = "1", required, string)]
    pub api : String,

    /// The workflow of which the task to execute is a part.
    #[prost(tag = "2", required, string)]
    pub workflow : String,
    /// The index of the task to execute in the workflow's task table.
    #[prost(tag = "3", required, uint64)]
    pub task     : u64,

    /// The arguments to run the request with. Given as a vector of JSON-encoded FullValue's.
    #[prost(tag = "4", repeated, string)]
    pub args : Vec<String>,
}

/// The reply sent by the worker while a task has executing.
#[derive(Clone, Message)]
pub struct TaskReply {
    /// The current status of the task. May also indicate a failure status.
    #[prost(tag = "1", required, enumeration = "TaskStatus")]
    pub status : i32,
    /// An optional value that may be carried along with some of the statusses.
    #[prost(tag = "2", optional, string)]
    pub value  : Option<String>,
}



/// Request for committing a result to a full dataset.
#[derive(Clone, Message)]
pub struct CommitRequest {

}

/// The reply sent by the worker when the comittation was successfull.
#[derive(Clone, Message)]
pub struct CommitReply {}
