//  LIB.rs
//    by Lut99
// 
//  Created:
//    24 Oct 2022, 15:26:59
//  Last edited:
//    14 Nov 2022, 11:01:20
//  Auto updated?
//    Yes
// 
//  Description:
//!   The `brane-tsk` library picks up where `brane-exe` left off, and
//!   implements various functions that can be used to manage tasks for
//!   various scenario's using within the Brane framework.
// 

// Declare modules
pub mod errors;
pub mod spec;
pub mod tools;
pub mod docker;
pub mod api;
pub mod offline;
pub mod instance;

// The grpc module is a bit special
#[allow(clippy::all)]
pub mod grpc {
    tonic::include_proto!("driver");
    tonic::include_proto!("job");

    pub use driver_service_client::DriverServiceClient;
    pub use driver_service_server::{DriverService, DriverServiceServer};
    pub use job_service_client::JobServiceClient;
    pub use job_service_server::{JobService, JobServiceServer};
}
