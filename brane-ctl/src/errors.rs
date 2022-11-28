//  ERRORS.rs
//    by Lut99
// 
//  Created:
//    21 Nov 2022, 15:46:26
//  Last edited:
//    28 Nov 2022, 13:24:05
//  Auto updated?
//    Yes
// 
//  Description:
//!   Defines the errors that may occur in the `brane-ctl` executable.
// 

use std::error::Error;
use std::fmt::{Display, Formatter, Result as FResult};
use std::path::PathBuf;
use std::process::{Command, ExitStatus};

use bollard::ClientVersion;
use console::style;

use brane_cfg::node::NodeKind;
use brane_shr::debug::EnumDebug;
use brane_tsk::docker::ImageSource;
use specifications::container::Image;


/***** LIBRARY *****/
/// Errors that relate to generating files.
#[derive(Debug)]
pub enum GenerateError {
    /// Failed to canonicalize the given path.
    CanonicalizeError{ path: PathBuf, err: std::io::Error },

    /// Failed to create a new file.
    FileCreateError{ path: PathBuf, err: std::io::Error },
    /// Failed to write the header to the new file.
    FileHeaderWriteError{ path: PathBuf, err: std::io::Error },
    /// Failed to write the main body to the new file.
    FileBodyWriteError{ path: PathBuf, err: brane_cfg::node::Error },
}
impl Display for GenerateError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FResult {
        use GenerateError::*;
        match self {
            CanonicalizeError{ path, err } => write!(f, "Failed to canonicalize path '{}': {}", path.display(), err),

            FileCreateError{ path, err }      => write!(f, "Failed to create new node.yml file '{}': {}", path.display(), err),
            FileHeaderWriteError{ path, err } => write!(f, "Failed to write header to node.yml file '{}': {}", path.display(), err),
            FileBodyWriteError{ err, .. }     => write!(f, "Failed to write body to node.yml file: {}", err),
        }
    }
}
impl Error for GenerateError {}



/// Errors that relate to managing the lifetime of the node.
#[derive(Debug)]
pub enum LifetimeError {
    /// Failed to canonicalize the given path.
    CanonicalizeError{ path: PathBuf, err: std::io::Error },

    /// Failed to open the extra hosts file.
    HostsFileCreateError{ path: PathBuf, err: std::io::Error },
    /// Failed to write to the extra hosts file.
    HostsFileWriteError{ path: PathBuf, err: serde_yaml::Error },

    /// Failed to get the digest of the given image file.
    ImageDigestError{ path: PathBuf, err: brane_tsk::docker::Error },
    /// Failed to load/import the given image.
    ImageLoadError{ image: Image, source: ImageSource, err: brane_tsk::docker::Error },

    /// Failed to load the given node config file.
    NodeConfigLoadError{ err: brane_cfg::node::Error },
    /// Failed to connect to the local Docker daemon.
    DockerConnectError{ socket: PathBuf, version: ClientVersion, err: bollard::errors::Error },
    /// The given start command (got) did not match the one in the `node.yml` file (expected).
    UnmatchedNodeKind{ got: NodeKind, expected: NodeKind },

    /// Failed to launch the given job.
    JobLaunchError{ command: Command, err: std::io::Error },
    /// The given job failed.
    JobFailure{ command: Command, status: ExitStatus },
}
impl Display for LifetimeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FResult {
        use LifetimeError::*;
        match self {
            CanonicalizeError{ path, err } => write!(f, "Failed to canonicalize path '{}': {}", path.display(), err),

            HostsFileCreateError{ path, err } => write!(f, "Failed to create extra hosts file '{}': {}", path.display(), err),
            HostsFileWriteError{ path, err }  => write!(f, "Failed to write to extra hosts file '{}': {}", path.display(), err),
    
            ImageDigestError{ path, err }        => write!(f, "Failed to get digest of image {}: {}", style(path.display()).bold(), err),
            ImageLoadError{ image, source, err } => write!(f, "Failed to load image {} from '{}': {}", style(image).bold(), style(source).bold(), err),

            NodeConfigLoadError{ err }                 => write!(f, "Failed to load node.yml file: {}", err),
            DockerConnectError{ socket, version, err } => write!(f, "Failed to connect to local Docker socket '{}' using API version {}: {}", socket.display(), version, err),
            UnmatchedNodeKind{ got, expected }         => write!(f, "Got command to start {} node, but 'node.yml' defined a {} node", got.variant(), expected.variant()),

            JobLaunchError{ command, err } => write!(f, "Failed to launch command '{:?}': {}", command, err),
            JobFailure{ command, status }  => write!(f, "Command '{}' failed with exit code {} (see output above)", style(format!("{:?}", command)).bold(), style(status.code().map(|c| c.to_string()).unwrap_or_else(|| "non-zero".into())).bold()),
        }
    }
}
impl Error for LifetimeError {}



/// Errors that relate to parsing Docker client version numbers.
#[derive(Debug)]
pub enum DockerClientVersionParseError {
    /// Missing a dot in the version number
    MissingDot{ raw: String },
    /// The given major version was not a valid usize
    IllegalMajorNumber{ raw: String, err: std::num::ParseIntError },
    /// The given major version was not a valid usize
    IllegalMinorNumber{ raw: String, err: std::num::ParseIntError },
}
impl Display for DockerClientVersionParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FResult {
        use DockerClientVersionParseError::*;
        match self {
            MissingDot{ raw }              => write!(f, "Missing '.' in Docket client version number '{}'", raw),
            IllegalMajorNumber{ raw, err } => write!(f, "'{}' is not a valid Docket client version major number: {}", raw, err),
            IllegalMinorNumber{ raw, err } => write!(f, "'{}' is not a valid Docket client version minor number: {}", raw, err),
        }
    }
}
impl Error for DockerClientVersionParseError {}



/// Errors that relate to parsing HostnamePairs.
#[derive(Debug)]
pub enum HostnamePairParseError {
    /// Missing a colon in the pair.
    MissingColon{ raw: String },
    /// Failed to parse the given IP as an IPv4 or an IPv6
    IllegalIpAddr{ raw: String, err: std::net::AddrParseError },
}
impl Display for HostnamePairParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FResult {
        use HostnamePairParseError::*;
        match self {
            MissingColon{ raw }       => write!(f, "Missing ':' in hostname/IP pair '{}'", raw),
            IllegalIpAddr{ raw, err } => write!(f, "Failed to parse '{}' as a valid IP address: {}", raw, err),
        }
    }
}
impl Error for HostnamePairParseError {}
