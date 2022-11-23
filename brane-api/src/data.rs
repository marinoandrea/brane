//  DATA.rs
//    by Lut99
// 
//  Created:
//    26 Sep 2022, 17:20:55
//  Last edited:
//    22 Nov 2022, 15:53:28
//  Auto updated?
//    Yes
// 
//  Description:
//!   Defines functions that handle REST-functions to the `/data` path and
//!   nested.
// 

use std::collections::HashMap;
use std::path::PathBuf;

use log::{debug, error};
use reqwest::{Client, ClientBuilder, Proxy, StatusCode};
use reqwest::tls::Certificate;
use warp::{Rejection, Reply};
use warp::http::{HeaderValue, Response};
use warp::hyper::Body;

use brane_cfg::{InfraFile, InfraPath};
use brane_cfg::certs::load_cert;
use brane_cfg::node::NodeConfig;
use brane_shr::utilities::is_ip_addr;
use specifications::data::{AssetInfo, DataInfo};

pub use crate::errors::DataError as Error;
use crate::spec::Context;


/***** HELPER MACROS *****/
/// Quits a path callback with a SecretError.
macro_rules! fail {
    () => {
        return Err(warp::reject::custom(Error::SecretError))
    };
}





/***** LIBRARY *****/
/// Lists the datasets that are known in the instance.
/// 
/// # Arguments
/// - `context`: The Context that contains stuff we need to run.
/// 
/// # Returns
/// A response that can be send to client. Specifically, it will contains a map (i.e., `HashMap`) of DataInfo structs that describe all the known datasets and where they live (mapped by their name).
/// 
/// # Errors
/// This function may error (i.e., reject the request) if we failed to load the infrastructure file.
pub async fn list(context: Context) -> Result<impl Reply, Rejection> {
    debug!("Handling GET on `/data/info` (i.e., list all datasets)...");

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

    // Iterate through all the locations (each of which have their own registry service)
    let mut datasets: HashMap<String, DataInfo> = HashMap::new();
    for (name, loc) in infra {
        // Load the certificates for this domain
        let root: Certificate = {
            // Load the root store for this location (also as a list of certificates)
            let cafile: PathBuf = node_config.node.central().paths.certs.join(&name).join("ca.pem");
            match load_cert(&cafile) {
                Ok(mut root) => if !root.is_empty() {
                    match Certificate::from_der(&root.swap_remove(0).0) {
                        Ok(root) => root,
                        Err(err) => {
                            error!("Failed to parse CA certificate file '{}' for location '{}': {} (skipping domain)", cafile.display(), name, err);
                            continue;
                        },
                    }
                } else {
                    error!("No certificates found in CA certificate file '{}' for location '{}' (skipping domain)", cafile.display(), name);
                    continue;
                },
                Err(err) => {
                    error!("Failed to load CA certificate file '{}' for location '{}': {} (skipping domain)", cafile.display(), name, err);
                    continue;
                },  
            }
        };

        // Determine if we have to enable SNI or not
        let address: String = format!("{}/data/info", loc.registry);
        let use_sni: bool = !is_ip_addr(&address);

        // Build a client with that certificate
        let mut client: ClientBuilder = Client::builder()
            .add_root_certificate(root)
            .tls_sni(use_sni);
        if let Some(proxy) = &node_config.proxy {
            client = client.proxy(match Proxy::all(proxy.serialize().to_string()) {
                Ok(proxy) => proxy,
                Err(err)  => {
                    error!("Failed to create proxy to '{}': {} (skipping domain)", proxy, err);
                    continue;
                },
            });
        }
        let client: Client = match client.build() {
            Ok(client) => client,
            Err(err)   => {
                error!("Failed to create client: {} (skipping domain)", err);
                continue;
            }
        };

        // Build the request
        let req: reqwest::Request = match client.get(&address).build() {
            Ok(res)  => res,
            Err(err) => {
                error!("Failed to create GET-request to '{}': {} (skipping domain)", address, err);
                continue;
            },
        };

        // Run a GET-request on `/data/info` to fetch all datasets known in this domain
        let res: reqwest::Response = match client.execute(req).await {
            Ok(res)  => res,
            Err(err) => {
                error!("{} (skipping domain)", Error::RequestError { address, err });
                continue;
            },
        };

        // Fetch the body
        let body: String = match res.text().await {
            Ok(body) => body,
            Err(err) => {
                error!("{} (skipping domain)", Error::ResponseBodyError{ address, err });
                continue;
            }
        };
        let local_sets: HashMap<String, AssetInfo> = match serde_json::from_str(&body) {
            Ok(body) => body,
            Err(err) => {
                debug!("Received body: \"\"\"{}\"\"\"", body);
                error!("{} (skipping domain)", Error::ResponseParseError{ address, err });
                continue;
            }  
        };

        // Merge that into the existing mapping of DataInfos
        for (n, d) in local_sets {
            if let Some(info) = datasets.get_mut(&n) {
                // Add this location
                info.access.insert(name.clone(), d.access);
            } else {
                datasets.insert(n, d.into_data_info(name.clone()));
            }
        }
    }

    // Now serialize this map
    let body: String = match serde_json::to_string(&datasets) {
        Ok(body) => body,
        Err(err) => {
            error!("{}", Error::SerializeError{ what: "list of all datasets", err });
            fail!();
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



/// Retrieves all information about the given dataset.
/// 
/// # Arguments
/// - `name`: The name of the dataset to query about.
/// - `context`: The Context that contains stuff we need to run.
/// 
/// # Returns
/// A response that can be send to client. Specifically, it will contains a DataInfo struct that describes everything we know about it.
/// 
/// # Errors
/// This function may error (i.e., reject the request) if the given name was not known.
pub async fn get(name: String, context: Context) -> Result<impl Reply, Rejection> {
    debug!("Handling GET on `/data/info/{}` (i.e., get dataset info)...", name);

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

    // Iterate through all the locations (each of which have their own registry service)
    let mut dataset: Option<DataInfo> = None;
    for (loc_name, loc) in infra {
        // Load the certificates for this domain
        let root: Certificate = {
            // Load the root store for this location (also as a list of certificates)
            let cafile: PathBuf = node_config.node.central().paths.certs.join(&name).join("ca.pem");
            match load_cert(&cafile) {
                Ok(mut root) => {
                    if !root.is_empty() {
                        match Certificate::from_der(&root.swap_remove(0).0) {
                            Ok(root) => root,
                            Err(err) => {
                                error!("Failed to parse CA certificate file '{}' for location '{}': {} (skipping domain)", cafile.display(), name, err);
                                continue;
                            },
                        }
                    } else {
                        error!("No certificates found in CA certificate file '{}' for location '{}' (skipping domain)", cafile.display(), name);
                        continue;
                    }
                },
                Err(err) => {
                    error!("Failed to load CA certificate file '{}' for location '{}': {} (skipping domain)", cafile.display(), name, err);
                    continue;
                },  
            }
        };

        // Determine if we have to enable SNI or not
        let address: String = format!("{}/data/info/{}", loc.registry, name);
        let use_sni: bool = !is_ip_addr(&address);

        // Build a client with that certificate
        let mut client: ClientBuilder = Client::builder()
            .add_root_certificate(root)
            .tls_sni(use_sni);
        if let Some(proxy) = &node_config.proxy {
            client = client.proxy(match Proxy::all(proxy.serialize().to_string()) {
                Ok(proxy) => proxy,
                Err(err)  => {
                    error!("Failed to create proxy to '{}': {} (skipping domain)", proxy, err);
                    continue;
                },
            });
        }
        let client: Client = match client.build() {
            Ok(client) => client,
            Err(err)   => {
                error!("Failed to create client: {} (skipping domain)", err);
                continue;
            }
        };

        // Build the request
        let req: reqwest::Request = match client.get(&address).build() {
            Ok(res)  => res,
            Err(err) => {
                error!("Failed to create GET-request to '{}': {} (skipping domain)", address, err);
                continue;
            },
        };

        // Run a GET-request on `/data` to fetch the specific dataset we're asked for
        let res: reqwest::Response = match client.execute(req).await {
            Ok(res)  => res,
            Err(err) => {
                error!("{} (skipping domain datasets)", Error::RequestError { address, err });
                continue;
            },
        };
        if res.status() == StatusCode::NOT_FOUND {
            // Search the next one instead
            continue;
        }

        // Fetch the body
        let body: String = match res.text().await {
            Ok(body) => body,
            Err(err) => {
                error!("{} (skipping domain datasets)", Error::ResponseBodyError{ address, err });
                continue;
            }
        };
        let local_set: AssetInfo = match serde_json::from_str(&body) {
            Ok(body) => body,
            Err(err) => {
                debug!("Received body: \"\"\"{}\"\"\"", body);
                error!("{} (skipping domain datasets)", Error::ResponseParseError{ address, err });
                continue;
            }  
        };

        // Either add or set that as the result
        if let Some(info) = &mut dataset {
            info.access.insert(loc_name, local_set.access);
        } else {
            dataset = Some(local_set.into_data_info(loc_name));
        }
    }

    // If we failed to find it, 404 as well
    if dataset.is_none() { return Err(warp::reject::not_found()); }

    // Now serialize this thing
    let body: String = match serde_json::to_string(&dataset) {
        Ok(body) => body,
        Err(err) => {
            error!("{}", Error::SerializeError{ what: "dataset metadata", err });
            fail!();
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
