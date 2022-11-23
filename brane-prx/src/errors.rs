//  ERRORS.rs
//    by Lut99
// 
//  Created:
//    23 Nov 2022, 11:43:56
//  Last edited:
//    23 Nov 2022, 14:57:44
//  Auto updated?
//    Yes
// 
//  Description:
//!   Defines the errors that may occur in the `brane-prx` crate.
// 

use std::error::Error;
use std::fmt::{Display, Formatter, Result as FResult};
use std::net::SocketAddr;

use brane_cfg::node::Address;


/***** LIBRARY *****/
/// Defines errors that relate to redirection.
#[derive(Debug)]
pub enum RedirectError {
    /// Asked to do TLS with an IP
    TlsWithNonHostnameError{ kind: String },
    /// The given hostname was illegal
    IllegalServerName{ raw: String, err: rustls::client::InvalidDnsNameError },
    /// Failed to create a new tcp listener.
    ListenerCreateError{ address: SocketAddr, err: std::io::Error },
    /// Failed to create a new socks client.
    SocksCreateError{ address: Address, err: anyhow::Error },

    /// Failed to connect using a regular ol' TcpStream.
    TcpStreamConnectError{ address: Address, err: std::io::Error },
    /// Failed to connect using a SOCKS6 client.
    Socks6ConnectError{ address: Address, proxy: Address, err: anyhow::Error },
}
impl Display for RedirectError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FResult {
        use RedirectError::*;
        match self {
            TlsWithNonHostnameError{ kind }     => write!(f, "Got a request for TLS but with a non-hostname {} address provided", kind),
            IllegalServerName{ raw, err }       => write!(f, "Cannot parse '{}' as a valid server name: {}", raw, err),
            ListenerCreateError{ address, err } => write!(f, "Failed to create new TCP listener on '{}': {}", address, err),
            SocksCreateError{ address, err }    => write!(f, "Failed to create new SOCKS6 client to '{}': {}", address, err),

            TcpStreamConnectError{ address, err }     => write!(f, "Failed to connect to '{}': {}", address, err),
            Socks6ConnectError{ address, proxy, err } => write!(f, "Failed to connect to '{}' through proxy '{}': {}", address, proxy, err),
        }
    }
}
impl Error for RedirectError {}
