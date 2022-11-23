//  INFRA.rs
//    by Lut99
// 
//  Created:
//    02 Nov 2022, 16:21:33
//  Last edited:
//    22 Nov 2022, 15:00:20
//  Auto updated?
//    Yes
// 
//  Description:
//!   Returns information about the infrastructure.
// 

use std::collections::HashMap;

use log::{debug, error};
use warp::{Reply, Rejection};
use warp::hyper::{Body, Response};
use warp::hyper::header::HeaderValue;

use brane_cfg::{InfraFile, InfraLocation, InfraPath};
use brane_cfg::node::NodeConfig;

pub use crate::errors::InfraError as Error;
use crate::spec::Context;


/***** LIBRARY *****/
/// Lists the registries at each location.
/// 
/// # Arguments
/// - `context`: The Context that contains stuff we need to run.
/// 
/// # Returns
/// A response that can be send to client. Specifically, it will contains a map (i.e., `HashMap`) of locations names to addresses where their registries may be found.
/// 
/// # Errors
/// This function may error (i.e., reject the request) if we failed to load the infrastructure file.
pub async fn registries(context: Context) -> Result<impl Reply, Rejection> {
    debug!("Handling GET on `/infra/registries` (i.e., list all regitsry endpoints)...");

    // Load the node config file
    let node_config: NodeConfig = match NodeConfig::from_path(&context.node_config_path) {
        Ok(config) => config,
        Err(err)   => {
            error!("Failed to load NodeConfig file: {}", err);
            return Err(warp::reject::custom(Error::SecretError));
        },
    };
    if !node_config.node.is_central() {
        error!("Provided node config file '{}' is not for a central node", context.node_config_path.display());
        return Err(warp::reject::custom(Error::SecretError));
    }

    // Load the infrastructure file
    let infra: InfraFile = match InfraFile::from_path(InfraPath::new(&node_config.node.central().paths.infra, &node_config.node.central().paths.secrets)) {
        Ok(infra) => infra,
        Err(err)  => {
            error!("{}", Error::InfrastructureOpenError{ path: node_config.node.central().paths.infra.clone(), err });
            return Err(warp::reject::custom(Error::SecretError));
        },
    };

    // Iterate through all of the regitries
    let mut locations: HashMap<String, String> = HashMap::new();
    for (name, loc) in infra.into_iter() {
        locations.insert(name, loc.registry);
    }

    // Now serialize this map
    let body: String = match serde_json::to_string(&locations) {
        Ok(body) => body,
        Err(err) => {
            error!("{}", Error::SerializeError{ what: "list of all registry endpoints", err });
            return Err(warp::reject::custom(Error::SecretError));
        }
    };
    let body_len: usize = body.len();

    // Create the respones around it
    let mut response = Response::new(Body::from(body));
    response.headers_mut().insert(
        "Content-Length",
        HeaderValue::from(body_len),
    );

    // Done
    Ok(response)
}



/// Returns the registry address for the requested location.
/// 
/// # Arguments
/// - `loc`: The location that the address is asked of.
/// - `context`: The Context that contains stuff we need to run.
/// 
/// # Returns
/// A response that can be send to client. Specifically, it will contains the address of the registry as plain text.
/// 
/// # Errors
/// This function may error (i.e., reject the request) if we failed to load the infrastructure file.
pub async fn get_registry(loc: String, context: Context) -> Result<impl Reply, Rejection> {
    debug!("Handling GET on `/infra/registries/{}` (i.e., get location registry address)...", loc);

    // Load the node config file
    let node_config: NodeConfig = match NodeConfig::from_path(&context.node_config_path) {
        Ok(config) => config,
        Err(err)   => {
            error!("Failed to load NodeConfig file: {}", err);
            return Err(warp::reject::custom(Error::SecretError));
        },
    };
    if !node_config.node.is_central() {
        error!("Provided node config file '{}' is not for a central node", context.node_config_path.display());
        return Err(warp::reject::custom(Error::SecretError));
    }

    // Load the infrastructure file
    let infra: InfraFile = match InfraFile::from_path(InfraPath::new(&node_config.node.central().paths.infra, &node_config.node.central().paths.secrets)) {
        Ok(infra) => infra,
        Err(err)  => {
            error!("{}", Error::InfrastructureOpenError{ path: node_config.node.central().paths.infra.clone(), err });
            return Err(warp::reject::custom(Error::SecretError));
        },
    };

    // Find the location requested
    let info: &InfraLocation = match infra.get(&loc) {
        Some(info) => info,
        None       => { return Err(warp::reject::not_found()); },
    };

    // Create a body with the registry
    let body     : String = info.registry.clone();
    let body_len : usize  = body.len();

    // Create the respones around it
    let mut response = Response::new(Body::from(body));
    response.headers_mut().insert(
        "Content-Length",
        HeaderValue::from(body_len),
    );

    // Done
    Ok(response)
}
