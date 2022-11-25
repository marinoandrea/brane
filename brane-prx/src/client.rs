//  CLIENT.rs
//    by Lut99
// 
//  Created:
//    25 Nov 2022, 15:09:17
//  Last edited:
//    25 Nov 2022, 16:09:17
//  Auto updated?
//    Yes
// 
//  Description:
//!   Provides client code for the `brane-prx` service. In particular,
//!   offers functionality for generating new paths.
// 

use std::str::FromStr;

use brane_cfg::node::Address;

use log::debug;
use reqwest::{Client, Response, Request};

use crate::spec::{NewPathRequest, NewPathRequestTlsOptions};

pub use crate::errors::ClientError as Error;


/***** LIBRARY *****/
/// Declares a new path in the proxy services.
/// 
/// # Arguments
/// - `endpoint`: The proxy service to connect to (hostname + address).
/// - `remote_address`: The remote address to connect to through the proxy.
/// - `tls`: If given, whether to use TLS and for what location.
/// 
/// # Returns
/// The port of the new path that is created.
/// 
/// # Errors
/// This function errors if we failed to create the port for whatever reason.
pub async fn create_path(endpoint: impl AsRef<Address>, remote: impl Into<String>, tls: Option<NewPathRequestTlsOptions>) -> Result<u16, Error> {
    let endpoint : &Address = endpoint.as_ref();
    let remote   : String   = remote.into();
    debug!("Creating path to '{}' on proxy service '{}'...", remote, endpoint);

    // Prepare the request
    let request: NewPathRequest = NewPathRequest {
        address : remote.into(),
        tls,
    };

    // Send it with reqwest
    let address : String = format!("{}/paths/new", endpoint);
    let client  : Client = Client::new();
    let req: Request = match client.post(&address).json(&request).build() {
        Ok(req)  => req,
        Err(err) => { return Err(Error::RequestBuildError{ address, err }); },
    };
    let res: Response = match client.execute(req).await {
        Ok(res)  => res,
        Err(err) => { return Err(Error::RequestError { address, err }); },
    };
    if !res.status().is_success() { return Err(Error::RequestFailure { address, code: res.status(), err: res.text().await.ok() }); }

    // Extract the port
    let port: String = match res.text().await {
        Ok(port) => port,
        Err(err) => { return Err(Error::RequestTextError{ address, err }); },
    };
    let port: u16 = match u16::from_str(&port) {
        Ok(port) => port,
        Err(err) => { return Err(Error::RequestPortParseError{ address, raw: port, err }); },
    };

    // Done
    Ok(port)
}
