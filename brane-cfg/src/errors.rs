//  ERRORS.rs
//    by Lut99
// 
//  Created:
//    04 Oct 2022, 11:09:56
//  Last edited:
//    16 Nov 2022, 17:41:06
//  Auto updated?
//    Yes
// 
//  Description:
//!   Defines errors that occur in the `brane-cfg` crate.
// 

use std::error::Error;
use std::fmt::{Display, Formatter, Result as FResult};
use std::path::PathBuf;


/***** LIBRARY *****/
/// Errors that relate to certificate loading and such.
#[derive(Debug)]
pub enum CertsError {
    /// Failed to open a given file.
    FileOpenError{ what: &'static str, path: PathBuf, err: std::io::Error },
    /// Failed to parse the certificate file.
    CertFileParseError{ path: PathBuf, err: std::io::Error },
    /// Failed to parse the key file.
    KeyFileParseError{ path: PathBuf, err: std::io::Error },
    /// The given certificate file was empty.
    EmptyCertFile{ path: PathBuf },
    /// The given keyfile was empty.
    EmptyKeyFile{ path: PathBuf },
}

impl Display for CertsError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FResult {
        use CertsError::*;
        match self {
            FileOpenError{ what, path, err } => write!(f, "Failed to open {} file '{}': {}", what, path.display(), err),
            CertFileParseError{ path, err }  => write!(f, "Failed to parse certificates in '{}': {}", path.display(), err),
            KeyFileParseError{ path, err }   => write!(f, "Failed to parse keys in '{}': {}", path.display(), err),
            EmptyCertFile{ path }            => write!(f, "No certificates found in certificate file '{}'", path.display()),
            EmptyKeyFile{ path }             => write!(f, "No keys found in keyfile '{}'", path.display()),
        }
    }
}

impl Error for CertsError {}



/// Errors that relate to resolving secrets.
#[derive(Debug)]
pub enum SecretsError {
    /// Failed to open the given file.
    FileOpenError{ path: PathBuf, err: std::io::Error },
    /// Failed to read/parse the given file as YAML.
    FileParseError{ path: PathBuf, err: serde_yaml::Error },

    /// The given location had no secrets defined in the secrets file.
    MissingSecret{ path: PathBuf, loc: String, what: &'static str },
    /// The given location had a secret specified that we cannot use.
    IncompatibleSecret{ path: PathBuf, loc: String, expected: &'static str, got: &'static str },
}

impl Display for SecretsError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FResult {
        use SecretsError::*;
        match self {
            FileOpenError{ path, err }       => write!(f, "Failed to open secrets file '{}': {}", path.display(), err),
            FileParseError{ path, err }      => write!(f, "Failed to parse secrets file '{}' as YAML: {}", path.display(), err),

            MissingSecret{ path, loc, what }               => write!(f, "Secrets file '{}' has no {} entry for location '{}'", path.display(), what, loc),
            IncompatibleSecret{ path, loc, expected, got } => write!(f, "Secrets file '{}' has an incompatible entry for location '{}': Expected {}, got {}", path.display(), loc, expected, got),
        }
    }
}

impl Error for SecretsError {}



/// Errors that relate to the InfraFile struct.
#[derive(Debug)]
pub enum InfraFileError {
    /// Failed to open the given file.
    FileOpenError{ path: PathBuf, err: std::io::Error },
    /// Failed to read/parse the given file as YAML.
    FileParseError{ path: PathBuf, err: serde_yaml::Error },

    /// Failed to resolve the secrets.
    SecretsResolveError{ path: PathBuf, err: SecretsError },
}

impl Display for InfraFileError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FResult {
        use InfraFileError::*;
        match self {
            FileOpenError{ path, err }  => write!(f, "Failed to open infrastructure file '{}': {}", path.display(), err),
            FileParseError{ path, err } => write!(f, "Failed to parse infrastructure file '{}' as YAML: {}", path.display(), err),

            SecretsResolveError{ path, err } => write!(f, "Failed to resolve secrets for infrastructure file '{}': {}", path.display(), err),
        }
    }
}

impl Error for InfraFileError {}



/// Errors that relate to the CredsFile struct.
#[derive(Debug)]
pub enum CredsFileError {
    /// Failed to open the given file.
    FileOpenError{ path: PathBuf, err: std::io::Error },
    /// Failed to read/parse the given file as YAML.
    FileParseError{ path: PathBuf, err: serde_yaml::Error },
}

impl Display for CredsFileError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FResult {
        use CredsFileError::*;
        match self {
            FileOpenError{ path, err }  => write!(f, "Failed to open credentials file '{}': {}", path.display(), err),
            FileParseError{ path, err } => write!(f, "Failed to parse credentials file '{}' as YAML: {}", path.display(), err),
        }
    }
}

impl Error for CredsFileError {}



/// Errors that relate to a NodeConfig.
#[derive(Debug)]
pub enum NodeConfigError {
    /// The given NodeKind was unknown to us.
    UnknownNodeKind{ raw: String },

    /// Failed to open the given config path.
    FileOpenError{ path: PathBuf, err: std::io::Error },
    /// Failed to read from the given config path.
    FileReadError{ path: PathBuf, err: std::io::Error },
    /// Failed to parse the given file.
    FileParseError{ path: PathBuf, err: serde_yaml::Error },

    /// Failed to open the given config path.
    FileCreateError{ path: PathBuf, err: std::io::Error },
    /// Failed to write to the given config path.
    FileWriteError{ path: PathBuf, err: std::io::Error },
    /// Failed to serialze the NodeConfig.
    ConfigSerializeError{ err: serde_yaml::Error },
}

impl Display for NodeConfigError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FResult {
        use NodeConfigError::*;
        match self {
            UnknownNodeKind{ raw } => write!(f, "Unknown node kind '{}'", raw),

            FileOpenError{ path, err }  => write!(f, "Failed to open the node config file '{}': {}", path.display(), err),
            FileReadError{ path, err }  => write!(f, "Failed to read the ndoe config file '{}': {}", path.display(), err),
            FileParseError{ path, err } => write!(f, "Failed to parse node config file '{}' as YAML: {}", path.display(), err),

            FileCreateError{ path, err } => write!(f, "Failed to create the node config file '{}': {}", path.display(), err),
            FileWriteError{ path, err }  => write!(f, "Failed to write to the ndoe config file '{}': {}", path.display(), err),
            ConfigSerializeError{ err }  => write!(f, "Failed to serialize node config to YAML: {}", err),
        }
    }
}

impl Error for NodeConfigError {}
