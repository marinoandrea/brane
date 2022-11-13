//  CERTS.rs
//    by Lut99
// 
//  Created:
//    02 Nov 2022, 11:47:55
//  Last edited:
//    02 Nov 2022, 13:54:25
//  Auto updated?
//    Yes
// 
//  Description:
//!   File that contains some useful functions for loading certificates
//!   and keys for `rustls`.
// 

use std::fs;
use std::io;
use std::path::Path;

use log::debug;
use rustls::{Certificate, PrivateKey, RootCertStore};
use rustls_pemfile::{certs, rsa_private_keys};

pub use crate::errors::CertsError as Error;


/***** LIBRARY *****/
/// Loads a given certificate file.
/// 
/// # Arguments
/// - `certfile`: Path to the certificate file to load.
/// 
/// # Returns
/// A nlist of all certificates found in the file. May be empty if we failed to parse any.
/// 
/// # Errors
/// This function errors if we failed to read the file.
pub fn load_cert(certfile: impl AsRef<Path>) -> Result<Vec<Certificate>, Error> {
    let certfile: &Path = certfile.as_ref();

    // Open a (buffered) file handle
    let handle: fs::File = match fs::File::open(certfile) {
        Ok(handle) => handle,
        Err(err)   => { return Err(Error::FileOpenError{ what: "certificate", path: certfile.into(), err }); },
    };
    let mut reader: io::BufReader<fs::File> = io::BufReader::new(handle);

    // Read the certificates in this file
    let certs: Vec<Vec<u8>> = match certs(&mut reader) {
        Ok(certs) => certs,
        Err(err)  => { return Err(Error::CertFileParseError{ path: certfile.into(), err }); },
    };
    debug!("Found {} certificate(s) in '{}'", certs.len(), certfile.display());

    // Done, return
    Ok(certs.into_iter().map(|c| Certificate(c)).collect())
}

/// Loads a given key file.
/// 
/// # Arguments
/// - `keyfile`: Path to the key file to load.
/// 
/// # Returns
/// A list of all keys found in the file. May be empty if we failed to parse any.
/// 
/// # Errors
/// This function errors if we failed to read the file.
pub fn load_key(keyfile: impl AsRef<Path>) -> Result<Vec<PrivateKey>, Error> {
    let keyfile: &Path = keyfile.as_ref();

    // Open a (buffered) file handle
    let handle: fs::File = match fs::File::open(keyfile) {
        Ok(handle) => handle,
        Err(err)   => { return Err(Error::FileOpenError{ what: "private key", path: keyfile.into(), err }); },
    };
    let mut reader: io::BufReader<fs::File> = io::BufReader::new(handle);

    // Read the certificates in this file
    let keys: Vec<Vec<u8>> = match rsa_private_keys(&mut reader) {
        Ok(keys) => keys,
        Err(err) => { return Err(Error::CertFileParseError{ path: keyfile.into(), err }); },
    };
    debug!("Found {} key(s) in '{}'", keys.len(), keyfile.display());

    // Done, return
    Ok(keys.into_iter().map(|k| PrivateKey(k)).collect())
}



/// Loads the server certificate / key pair from disk.
/// 
/// # Arguments
/// - `certfile`: Path to the certificate file to load.
/// - `keyfile`: Path to the keyfile to load.
/// 
/// # Returns
/// A new pair of certificates and the key.
/// 
/// # Errors
/// This function errors if we failed to read either of the files.
pub fn load_keypair(certfile: impl AsRef<Path>, keyfile: impl AsRef<Path>) -> Result<(Certificate, PrivateKey), Error> {
    let certfile : &Path = certfile.as_ref();
    let keyfile  : &Path = keyfile.as_ref();

    // Read the certificate first, then the key
    let mut certs : Vec<Certificate> = load_cert(certfile)?;
    let mut keys  : Vec<PrivateKey>  = load_key(keyfile)?;

    // We only continue with the first certificate and key
    let cert: Certificate = if !certs.is_empty() {
        certs.swap_remove(0)
    } else {
        return Err(Error::EmptyCertFile{ path: certfile.into() });
    };
    let key: PrivateKey = if !keys.is_empty() {
        keys.swap_remove(0)
    } else {
        return Err(Error::EmptyKeyFile{ path: keyfile.into() });
    };

    // Done, return
    Ok((cert, key))
}

/// Loads the client certificates from disk as a CertStore.
/// 
/// # Arguments
/// - `storefile`: Path to the certificate file to load.
/// 
/// # Returns
/// A new RootCertStore with the certificates of the allowed client.
/// 
/// # Errors
/// This function errors if we failed to read either of the files.
pub fn load_certstore(storefile: impl AsRef<Path>) -> Result<RootCertStore, Error> {
    let storefile : &Path = storefile.as_ref();

    // Read the certificate first
    let handle: fs::File = match fs::File::open(storefile) {
        Ok(handle) => handle,
        Err(err)   => { return Err(Error::FileOpenError{ what: "client certificate store", path: storefile.into(), err }); },
    };
    let mut reader: io::BufReader<fs::File> = io::BufReader::new(handle);

    // Read the certificates in this file
    let certs: Vec<Vec<u8>> = match certs(&mut reader) {
        Ok(certs) => certs,
        Err(err)  => { return Err(Error::CertFileParseError{ path: storefile.into(), err }); },
    };
    debug!("Found {} certificate(s) in '{}'", certs.len(), storefile.display());

    // Read the certificates in the file to the store.
    let mut store: RootCertStore = RootCertStore::empty();
    let (added, ignored): (usize, usize) = store.add_parsable_certificates(&certs);
    debug!("Created client trust store with {} certificates (ignored {})", added, ignored);

    // Done, for now
    Ok(store)
}
