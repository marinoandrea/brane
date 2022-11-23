//  MANAGE.rs
//    by Lut99
// 
//  Created:
//    23 Nov 2022, 11:07:05
//  Last edited:
//    23 Nov 2022, 12:50:46
//  Auto updated?
//    Yes
// 
//  Description:
//!   Defines warp-paths that relate to management of the proxy service.
// 

use std::error::Error;
use std::fmt::{Display, Formatter, Result as FResult};
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::{Arc, MutexGuard};

use log::{debug, error, info};
use warp::{Rejection, Reply};
use warp::http::StatusCode;
use warp::hyper::{Body, Response};
use warp::hyper::body::Bytes;

use crate::spec::{Context, NewPathRequest};
use crate::ports::PortAllocator;
use crate::redirect::path_server_factory;


/***** HELPER MACROS *****/
/// "Casts" the given StatusCode to an empty response.
macro_rules! response {
    (StatusCode::$status:ident) => {
        Response::builder().status(StatusCode::$status).body(Body::empty()).unwrap()
    };
}

/// "Casts" the given StatusCode to an empty response.
macro_rules! reject {
    ($msg:expr) => {
        {
            #[derive(Debug)]
            struct InternalError;
            impl Display for InternalError {
                fn fmt(&self, f: &mut Formatter<'_>) -> FResult {
                    write!(f, "An internal error has occurred.")
                }
            }
            impl Error for InternalError {}
            impl warp::reject::Reject for InternalError {}

            // Return that
            warp::reject::custom(InternalError)
        }
    };
}





/***** LIBRARY *****/
/// Creates a new path in the proxy service, returning the port on which it becomes available.
/// 
/// # Arguments
/// - `body`: The body of the given request, which we will attempt to parse as JSON.
/// - `context`: The Context struct that contains things we might need.
/// 
/// # Returns
/// A reponse with the following codes:
/// - `200 OK` if the new path was successfully created. In the body, there is the (serialized) port number of the path to store.
/// - `400 BAD REQUEST` if the given request body was not parseable as the desired JSON.
/// - `507 INSUFFICIENT STORAGE` if the server is out of port ranges to allocate.
/// 
/// # Errors
/// This function errors if we failed to start a new task that listens for the given port. If so, a `500 INTERNAL ERROR` is returned.
pub async fn new_path(body: Bytes, context: Arc<Context>) -> Result<impl Reply, Rejection> {
    info!("Handling POST on '/paths/new' (i.e., create new proxy path)...");

    // Start by parsing the incoming body
    debug!("Parsing incoming body...");
    let body: NewPathRequest = match serde_json::from_slice(&body) {
        Ok(body) => body,
        Err(err) => {
            error!("Failed to parse incoming request body as JSON: {}", err);
            return Ok(response!(StatusCode::BAD_REQUEST));
        },
    };

    // Attempt to find a free port in the allocator
    debug!("Finding available port...");
    let port: u16 = {
        let mut lock: MutexGuard<PortAllocator> = context.ports.lock().unwrap();
        match lock.allocate() {
            Some(port) => port,
            None       => {
                error!("No more ports left in range");
                return Ok(response!(StatusCode::INSUFFICIENT_STORAGE));
            },
        }
    };
    debug!("Allocating on: {}", port);

    // Create the future with those settings
    debug!("Launching service...");
    let address: SocketAddr = SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), port).into();
    let server = match path_server_factory(&context, address, body.address, body.tls).await {
        Ok(server) => server,
        Err(err)   => {
            error!("Failed to create the path server: {}", err);
            return Err(reject!("An internal server error has occurred."));
        },
   };
    // Spawn it as a separate task
    tokio::spawn(server);

    // Done, return the port
    debug!("OK, returning port to client");
    Ok(Response::new(Body::from(port.to_string())))
}
