//  ERRORS.rs
//    by Lut99
// 
//  Created:
//    04 Feb 2022, 10:35:12
//  Last edited:
//    02 Nov 2022, 16:29:22
//  Auto updated?
//    Yes
// 
//  Description:
//!   Contains general errors for across the brane-api package.
// 

use std::error::Error;
use std::fmt::{Display, Formatter, Result as FResult};
use std::path::PathBuf;

use scylla::transport::errors::NewSessionError;


/***** ERRORS *****/
/// Collects errors for the most general case in the brane-api package
#[derive(Debug)]
pub enum ApiError {
    /// Could not create a Scylla session
    ScyllaConnectError{ host: String, err: NewSessionError },
}

impl Display for ApiError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FResult {
        match self {
            ApiError::ScyllaConnectError{ host, err } => write!(f, "Could not connect to Scylla host '{}': {}", host, err),
        }
    }
}

impl Error for ApiError {}



/// Contains errors relating to the `/infra` path (and nested).
#[derive(Debug)]
pub enum InfraError {
    /// Failed to open/load the infrastructure file.
    InfrastructureOpenError{ path: PathBuf, err: brane_cfg::Error },
    /// Failed to serialize the response body.
    SerializeError{ what: &'static str, err: serde_json::Error },

    /// An internal error occurred that we would not like to divulge.
    SecretError,
}

impl Display for InfraError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FResult {
        use InfraError::*;
        match self {
            InfrastructureOpenError{ path, err }           => write!(f, "Failed to open infrastructure file '{}': {}", path.display(), err),
            SerializeError{ what, err }        => write!(f, "Failed to serialize {}: {}", what, err),

            SecretError => write!(f, "An internal error has occurred"),
        }
    }
}

impl Error for InfraError {}

impl warp::reject::Reject for InfraError {}



/// Contains errors relating to the `/data` path (and nested).
#[derive(Debug)]
pub enum DataError {
    /// Failed to open/load the infrastructure file.
    InfrastructureOpenError{ path: PathBuf, err: brane_cfg::Error },
    /// Failed to get the list of all locations.
    InfrastructureLocationsError{ path: PathBuf, err: brane_cfg::Error },
    /// Failed to get the metadata of a location.
    InfrastructureMetadataError{ path: PathBuf, name: String, err: brane_cfg::Error },

    /// Failed to send a GET-request to the given URL
    RequestError{ address: String, err: reqwest::Error },
    /// Failed to get the body of a response.
    ResponseBodyError{ address: String, err: reqwest::Error },
    /// Failed to parse the body of a response.
    ResponseParseError{ address: String, err: serde_json::Error },
    /// Failed to serialize the response body.
    SerializeError{ what: &'static str, err: serde_json::Error },

    /// An internal error occurred that we would not like to divulge.
    SecretError,
}

impl Display for DataError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FResult {
        use DataError::*;
        match self {
            InfrastructureOpenError{ path, err }           => write!(f, "Failed to open infrastructure file '{}': {}", path.display(), err),
            InfrastructureLocationsError{ path, err }      => write!(f, "Failed to get locations from infrastructure file '{}': {}", path.display(), err),
            InfrastructureMetadataError{ path, name, err } => write!(f, "Failed to get metadata of location '{}' from infrastructure file '{}': {}", name, path.display(), err),

            RequestError{ address, err }       => write!(f, "Failed to send GET-request to '{}': {}", address, err),
            ResponseBodyError{ address, err }  => write!(f, "Failed to get the response body received from '{}': {}", address, err),
            ResponseParseError{ address, err } => write!(f, "Failed to parse response from '{}' as JSON: {}", address, err),
            SerializeError{ what, err }        => write!(f, "Failed to serialize {}: {}", what, err),

            SecretError => write!(f, "An internal error has occurred"),
        }
    }
}

impl Error for DataError {}

impl warp::reject::Reject for DataError {}
