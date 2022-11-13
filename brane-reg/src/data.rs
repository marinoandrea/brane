//  DATA.rs
//    by Lut99
// 
//  Created:
//    26 Sep 2022, 15:40:40
//  Last edited:
//    13 Nov 2022, 15:19:45
//  Auto updated?
//    Yes
// 
//  Description:
//!   Defines functions that handle various REST-functions on the `/data`
//!   path (and children).
// 

use std::path::{Path, PathBuf};
use std::sync::Arc;

use log::{debug, error, info};
use rustls::Certificate;
use tempfile::TempDir;
use tokio::fs as tfs;
use tokio::io::AsyncReadExt;
use warp::{Rejection, Reply};
use warp::http::HeaderValue;
use warp::hyper::{Body, StatusCode};
use warp::hyper::body::{Bytes, Sender};
use warp::reply::{self, Response};
use x509_parser::certificate::X509Certificate;
use x509_parser::prelude::FromDer;

use brane_shr::fs::archive_async;
use specifications::data::{AccessKind, AssetInfo};

pub use crate::errors::DataError as Error;
use crate::errors::AuthorizeError;
use crate::spec::Context;
use crate::store::Store;


/***** HELPER FUNCTIONS *****/
/// Retrieves the client name from the given Certificate provided by the, well, client.
/// 
/// # Arguments
/// - `certificate`: The Certificate to analyze.
/// 
/// # Returns
/// The name of the client, as provided by the Certificate's `CN` field.
/// 
/// # Errors
/// This function errors if we could not extract the name for some reason. You should consider the client unauthenticated, in that case.
pub fn extract_client_name(cert: Certificate) -> Result<String, AuthorizeError> {
    // Attempt to parse the certificate as a real x509 one
    match X509Certificate::from_der(&cert.0) {
        Ok((_, cert)) => {
            // Get the part after 'CN = ' and before end-of-string or comma (since that's canonically the domain name)
            let subject: String = cert.subject.to_string();
            let name_loc: usize = match subject.find("CN=") {
                Some(name_loc) => name_loc + 3,
                None           => { return Err(AuthorizeError::ClientCertNoCN { subject }); },
            };
            let name_end: usize = subject[name_loc..].find(',').map(|c| name_loc + c).unwrap_or(subject.len());

            // Extract it as the name
            Ok(subject[name_loc..name_end].to_string())
        },
        Err(err) => Err(AuthorizeError::ClientCertParseError{ err }),
    }
}



/// Runs the do-be-done data transfer by the checker to assess if we're allowed to do it.
/// 
/// # Arguments
/// - `identity`: The name (or other method of identifying the user) of the person who will download the dataset.
/// - `data`: The name of the dataset they are trying to access.
/// 
/// # Returns
/// Whether permission is given or not.
/// 
/// # Errors
/// This function errors if we failed to ask the checker. Clearly, that should be treated as permission denied.
pub async fn assert_data_permission(identifier: impl AsRef<str>, data: impl AsRef<str>) -> Result<bool, AuthorizeError> {
    let identifier : &str = identifier.as_ref();
    let data       : &str = data.as_ref();

    // We don't have a checker yet to ask ;(

    // Instead, consider a few hardcoded policies:

    // 1. Finally, if we have the global results, the client (let's call them Rosanne) may download it
    if identifier == "rosanne" && data == "surf_result" { return Ok(true); }

    // Otherwise, permission _not_ allowed
    Ok(false)
}



/// Runs the do-be-done intermediate result transfer by the checker to assess if we're allowed to do it.
/// 
/// # Arguments
/// - `identity`: The name (or other method of identifying the user) of the person who will download the intermediate result.
/// - `result`: The name of the intermediate result they are trying to access.
/// 
/// # Returns
/// Whether permission is given or not.
/// 
/// # Errors
/// This function errors if we failed to ask the checker. Clearly, that should be treated as permission denied.
pub async fn assert_result_permission(identifier: impl AsRef<str>, _result: impl AsRef<str>) -> Result<bool, AuthorizeError> {
    let identifier : &str = identifier.as_ref();

    // We don't have a checker yet to ask ;(

    // Instead, consider a few hardcoded policies:

    // 1. SURF may download any result LOOOOL (this is because we can't really distinguish results beforehand yet...)
    if identifier == "surf" { return Ok(true); }

    // Otherwise, permission _not_ allowed
    Ok(false)
}





/***** LIBRARY *****/
/// Handles a GET on the main `/data` path, returning a JSON with the datasets known to this registry.
/// 
/// # Arguments
/// - `context`: The context that carries options and some shared structures between the warp paths.
/// 
/// # Returns
/// The response that can be send back to the client. Contains a JSON-encoded list (`Vec`) of AssetInfo structs.
/// 
/// # Errors
/// This function may error (i.e., reject) if we could not serialize the given store.
pub async fn list(context: Arc<Context>) -> Result<impl Reply, Rejection> {
    debug!("Handling GET on `/data/info` (i.e., list all datasets)...");

    // Load the store
    let store: Store = match Store::from_dirs(&context.data_path, &context.results_path).await {
        Ok(store) => store,
        Err(err)  => {
            error!("Failed to load the store: {}", err);
            return Err(warp::reject::reject());
        }
    };

    // Simply parse to a string
    let body: String = match serde_json::to_string(&store.datasets) {
        Ok(body) => body,
        Err(err) => {
            return Err(warp::reject::custom(Error::StoreSerializeError { err }));
        }
    };
    let body_len: usize = body.len();

    // Construct a response with the body and the content-length header
    let mut response = Response::new(Body::from(body));
    response.headers_mut().insert(
        "Content-Length",
        HeaderValue::from(body_len),
    );

    // Done
    Ok(response)
}



/// Handles a GET on a specific datasets in a child-path of the `/data`-path, returning a JSON with more information about this dataset.
/// 
/// # Arguments
/// - `name`: The name of the dataset to retrieve the metadata for.
/// - `context`: The context that carries options and some shared structures between the warp paths.
/// 
/// # Returns
/// The response that can be send back to the client. Contains a JSON-encoded AssetInfo struct with the metadata.
/// 
/// # Errors
/// This function may error (i.e., reject) if we didn't know the given name or we failred to serialize the relevant AssetInfo.
pub async fn get(name: String, context: Arc<Context>) -> Result<impl Reply, Rejection> {
    debug!("Handling GET on `/data/info/{}` (i.e., get dataset metdata)...", name);

    // Load the store
    let store: Store = match Store::from_dirs(&context.data_path, &context.results_path).await {
        Ok(store) => store,
        Err(err)  => {
            error!("Failed to load the store: {}", err);
            return Err(warp::reject::reject());
        }
    };

    // Attempt to resolve the name in the given store
    let info: &AssetInfo = match store.get_data(&name) {
        Some(info) => info,
        None       => {
            error!("Unknown dataset '{}'", name);
            return Err(warp::reject::not_found());
        },
    };

    // Serialize it (or at least, try so)
    let body: String = match serde_json::to_string(info) {
        Ok(body) => body,
        Err(err) => {
            return Err(warp::reject::custom(Error::AssetSerializeError { name, err }));
        },
    };
    let body_len: usize = body.len();

    // Construct a response with the body and the content-length header
    let mut response = Response::new(Body::from(body));
    response.headers_mut().insert(
        "Content-Length",
        HeaderValue::from(body_len),
    );

    // Done
    Ok(response)
}



/// Handles a GET that downloads an entire dataset. This basically emulates a data transfer.
/// 
/// # Arguments
/// - `cert`: The client certificate by which we may extract some identity. Only clients that are authenticated by the local store may connect.
/// - `name`: The name of the dataset to download.
/// - `context`: The context that carries options and some shared structures between the warp paths.
/// 
/// # Returns
/// The response that can be sent back to the client. Contains a raw binary of the dataset, which is packaged as an archive before sending.
/// 
/// # Errors
/// This function may error (i.e., reject) if we didn't know the given name or we failed to serialize the relevant AssetInfo.
pub async fn download_data(cert: Certificate, name: String, context: Arc<Context>) -> Result<impl Reply, Rejection> {
    debug!("Handling GET on `/data/download/{}` (i.e., download dataset)...", name);

    // Load the store
    let store: Store = match Store::from_dirs(&context.data_path, &context.results_path).await {
        Ok(store) => store,
        Err(err)  => {
            error!("Failed to load the store: {}", err);
            return Err(warp::reject::reject());
        }
    };

    // Attempt to resolve the name in the given store
    let info: &AssetInfo = match store.get_data(&name) {
        Some(info) => info,
        None       => {
            error!("Unknown dataset '{}'", name);
            return Err(warp::reject::not_found());
        },
    };

    // Attempt to parse the certificate to get the client's name (which tracks because it's already authenticated)
    let client_name: String = match extract_client_name(cert) {
        Ok(name) => name,
        Err(err) => {
            error!("{} (client unauthenticated)", err);
            return Ok(reply::with_status(Response::new(Body::empty()), StatusCode::FORBIDDEN));
        },
    };

    // Before we continue, assert that this dataset may be downloaded by this person (uh-oh, how we gon' do that)
    match assert_data_permission(&client_name, &info.name).await {
        Ok(true)  => {
            info!("Checker authorized download of dataset '{}' by '{}'", info.name, client_name);
        },

        Ok(false) => {
            info!("Checker denied download of dataset '{}' by '{}'", info.name, client_name);
            return Ok(reply::with_status(Response::new(Body::empty()), StatusCode::FORBIDDEN));
        },
        Err(err) => {
            error!("Failed to consult the checker: {}", err);
            return Err(warp::reject::reject());
        },
    }

    // Access the dataset in the way it likes to be accessed
    match &info.access {
        AccessKind::File { path } => {
            debug!("Accessing file '{}' @ '{}' as AccessKind::File...", name, path.display());
            let path: PathBuf = context.data_path.join(&name).join(path);
            debug!("File can be found under: '{}'", path.display());

            // First, get a temporary directory
            let tmpdir: TempDir = match TempDir::new() {
                Ok(tmpdir) => tmpdir,
                Err(err)   => {
                    let err = Error::TempDirCreateError{ err };
                    error!("{}", err);
                    return Err(warp::reject::custom(err));
                }
            };

            // Next, create an archive in the temporary directory
            let tar_path: PathBuf = tmpdir.path().join("data.tar.gz");
            if let Err(err) = archive_async(&path, &tar_path, true).await {
                let err = Error::DataArchiveError{ err };
                error!("{}", err);
                return Err(warp::reject::custom(err));
            }

            // Now we send the tarball as a file in the reply
            debug!("Sending back reply with compressed archive...");
            let (mut body_sender, body): (Sender, Body) = Body::channel();

            // Spawn a future that reads the file chunk-by-chunk (in case of large files)
            tokio::spawn(async move {
                // We move the temporary directory here just to keep it in scope
                let _tmpdir: TempDir = tmpdir;

                // Open the archive file to read
                let mut handle: tfs::File = match tfs::File::open(&tar_path).await {
                    Ok(handle) => handle,
                    Err(err)   => {
                        let err = Error::TarOpenError{ path: tar_path, err };
                        error!("{}", err);
                        return Err(warp::reject::custom(err));
                    },
                };

                // Read it chunk-by-chunk
                // (The size of the buffer, like most of the code but edited for not that library cuz it crashes during compilation, has been pulled from https://docs.rs/stream-body/latest/stream_body/)
                let mut buf: [u8; 1024 * 16] = [0; 1024 * 16];
                loop {
                    // Read the chunk
                    let bytes: usize = match handle.read(&mut buf).await {
                        Ok(bytes) => bytes,
                        Err(err)  => {
                            error!("{}", Error::TarReadError{ path: tar_path, err });
                            break;
                        },
                    };
                    if bytes == 0 { break; }

                    // Send that with the body
                    if let Err(err) = body_sender.send_data(Bytes::copy_from_slice(&buf[..bytes])).await {
                        error!("{}", Error::TarSendError{ err });
                    }
                }

                // Done
                Ok(())
            });

            // We use the handle as a stream.
            Ok(reply::with_status(Response::new(body), StatusCode::OK))
        },
    }
}

/// Handles a GET that downloads an intermediate result. This basically emulates a data transfer.
/// 
/// # Arguments
/// - `cert`: The client certificate by which we may extract some identity. Only clients that are authenticated by the local store may connect.
/// - `name`: The name of the intermediate result to download.
/// - `context`: The context that carries options and some shared structures between the warp paths.
/// 
/// # Returns
/// The response that can be sent back to the client. Contains a raw binary of the result, which is packaged as an archive before sending.
/// 
/// # Errors
/// This function may error (i.e., reject) if we didn't know the given name or we failed to serialize the relevant AssetInfo.
pub async fn download_result(cert: Certificate, name: String, context: Arc<Context>) -> Result<impl Reply, Rejection> {
    debug!("Handling GET on `/results/download/{}` (i.e., download intermediate result)...", name);

    // Load the store
    let store: Store = match Store::from_dirs(&context.data_path, &context.results_path).await {
        Ok(store) => store,
        Err(err)  => {
            error!("Failed to load the store: {}", err);
            return Err(warp::reject::reject());
        }
    };

    // Attempt to resolve the name in the given store
    let path: &Path = match store.get_result(&name) {
        Some(path) => path,
        None       => {
            error!("Unknown intermediate result '{}'", name);
            return Err(warp::reject::not_found());
        },
    };

    // Attempt to parse the certificate to get the client's name (which tracks because it's already authenticated)
    let client_name: String = match extract_client_name(cert) {
        Ok(name) => name,
        Err(err) => {
            error!("{} (client unauthenticated)", err);
            return Ok(reply::with_status(Response::new(Body::empty()), StatusCode::FORBIDDEN));
        },
    };

    // Before we continue, assert that this dataset may be downloaded by this person (uh-oh, how we gon' do that)
    match assert_result_permission(&client_name, &name).await {
        Ok(true)  => {
            info!("Checker authorized download of intermediate result '{}' by '{}'", name, client_name);
        },

        Ok(false) => {
            info!("Checker denied download of intermediate result '{}' by '{}'", name, client_name);
            return Ok(reply::with_status(Response::new(Body::empty()), StatusCode::FORBIDDEN));
        },
        Err(err) => {
            error!("Failed to consult the checker: {}", err);
            return Err(warp::reject::reject());
        },
    }

    // Start the upload; first, get a temporary directory
    let tmpdir: TempDir = match TempDir::new() {
        Ok(tmpdir) => tmpdir,
        Err(err)   => {
            let err = Error::TempDirCreateError{ err };
            error!("{}", err);
            return Err(warp::reject::custom(err));
        }
    };

    // Next, create an archive in the temporary directory
    let tar_path: PathBuf = tmpdir.path().join("data.tar.gz");
    if let Err(err) = archive_async(&path, &tar_path, true).await {
        let err = Error::DataArchiveError{ err };
        error!("{}", err);
        return Err(warp::reject::custom(err));
    }

    // Now we send the tarball as a file in the reply
    debug!("Sending back reply with compressed archive...");
    let (mut body_sender, body): (Sender, Body) = Body::channel();

    // Spawn a future that reads the file chunk-by-chunk (in case of large files)
    tokio::spawn(async move {
        // We move the temporary directory here just to keep it in scope
        let _tmpdir: TempDir = tmpdir;

        // Open the archive file to read
        let mut handle: tfs::File = match tfs::File::open(&tar_path).await {
            Ok(handle) => handle,
            Err(err)   => {
                let err = Error::TarOpenError{ path: tar_path, err };
                error!("{}", err);
                return Err(warp::reject::custom(err));
            },
        };

        // Read it chunk-by-chunk
        // (The size of the buffer, like most of the code but edited for not that library cuz it crashes during compilation, has been pulled from https://docs.rs/stream-body/latest/stream_body/)
        let mut buf: [u8; 1024 * 16] = [0; 1024 * 16];
        loop {
            // Read the chunk
            let bytes: usize = match handle.read(&mut buf).await {
                Ok(bytes) => bytes,
                Err(err)  => {
                    error!("{}", Error::TarReadError{ path: tar_path, err });
                    break;
                },
            };
            if bytes == 0 { break; }

            // Send that with the body
            if let Err(err) = body_sender.send_data(Bytes::copy_from_slice(&buf[..bytes])).await {
                error!("{}", Error::TarSendError{ err });
            }
        }

        // Done
        Ok(())
    });

    // We use the handle as a stream.
    Ok(reply::with_status(Response::new(body), StatusCode::OK))
}
