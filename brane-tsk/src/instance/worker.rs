//  WORKER.rs
//    by Lut99
// 
//  Created:
//    31 Oct 2022, 11:21:14
//  Last edited:
//    16 Nov 2022, 14:01:36
//  Auto updated?
//    Yes
// 
//  Description:
//!   Implements the worker side of the communication. This is the other
//!   side for all sorts of things, from execution to preprocessing to
//!   execution to publicizing/committing.
// 

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use base64ct::{Base64, Encoding};
use bollard::{API_DEFAULT_VERSION, ClientVersion};
use chrono::Utc;
use futures_util::StreamExt;
use hyper::body::Bytes;
use log::{debug, error, info, warn};
use reqwest::{Certificate, Client, ClientBuilder, Identity, Proxy};
use serde_json_any_key::json_to_map;
use sha2::{Digest, Sha256};
use tokio::fs as tfs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc::{self, Sender};
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Response, Request, Status};

use brane_ast::Workflow;
use brane_ast::locations::Location;
use brane_ast::ast::DataName;
use brane_cfg::CredsFile;
use brane_cfg::creds::Credentials;
use brane_exe::FullValue;
use brane_shr::debug::BlockFormatter;
use brane_shr::fs::{copy_dir_recursively_async, unarchive_async};
use specifications::container::{Image, VolumeBind};
use specifications::data::{AccessKind, AssetInfo};
use specifications::package::{PackageIndex, PackageInfo, PackageKind};
use specifications::version::Version;

use crate::errors::{AuthorizeError, CommitError, ExecuteError, PreprocessError};
use crate::spec::JobStatus;
use crate::grpc::{CommitReply, CommitRequest, DataKind, JobService, PreprocessKind, PreprocessReply, PreprocessRequest, TaskReply, TaskRequest, TaskStatus};
use crate::tools::decode_base64;
use crate::api::get_package_index;
use crate::docker::{self, ExecuteInfo, Network};


/***** CONSTANTS *****/
/// Path to the temporary folder.
pub const TEMPORARY_DIR: &str = "/tmp";





/***** HELPER MACROS *****/
/// Translates the given error into a log message, updates the client _and_ returns it.
macro_rules! err {
    ($tx:ident, $err:expr) => {
        err!($tx, JobStatus::CreationFailed, $err)
    };

    ($tx:ident, JobStatus::$status:ident, $err:expr) => {
        {
            let err = $err;
            log::error!("{}", err);
            if let Err(err) = update_client(&$tx, JobStatus::$status(format!("{}", err))).await { log::error!("{}", err); }
            Err(err)
        }
    };
}





/***** HELPER STRUCTURES *****/
/// Helper structure for grouping together worker-local "constants".
#[derive(Clone, Debug)]
pub struct EnvironmentInfo {
    /// The ID of this location.
    pub location_id : String,

    /// Path to the credentials file.
    pub creds_path        : PathBuf,
    /// The path to the directory with all certificates.
    pub certs_path        : PathBuf,
    /// The path to the folder with all the containers.
    pub packages_path     : PathBuf,
    /// The path to the folder with all the data.
    pub data_path         : PathBuf,
    // The path to the folder with all the results.
    pub results_path      : PathBuf,
    /// The path to store all temporarily downloaded datasets from other domains.
    pub temp_data_path    : PathBuf,
    /// The path to store all temporarily downloaded intermediate results from other domains.
    pub temp_results_path : PathBuf,

    /// The location of the checker service.
    pub checker_endpoint : String,
    /// If we should sent our data transfers through a proxy, this defines where to find it.
    pub proxy_address    : Option<String>,

    /// Whether to keep containers around after execution or not.
    pub keep_container : bool,
}
impl EnvironmentInfo {
    /// Constructor for the EnvironmentInfo.
    /// 
    /// # Arguments
    /// - `location_id`: The ID of this location.
    /// - `creds_path: Path to the credentials file.
    /// - `certs_path`: The path to the directory with all certificates.
    /// - `packages_path`: The path to the folder with all the containers.
    /// - `data_path`: The path to the folder with all the data.
    /// - `results_path`: The path to the folder with all the results.
    /// - `temp_data_path`: The path to store all temporarily downloaded datasets from other domains.
    /// - `temp_results_path`: The path to store all temporarily downloaded intermediate results from other domains.
    /// - `checker_endpoint`: The location of the checker service.
    /// - `proxy_address`: If we should sent our data transfers through a proxy, this defines where to find it.
    /// - `keep_container`: Whether to keep containers around after execution or not (useful for debugging containers).
    /// 
    /// # Returns
    /// A new EnvironmentInfo instance.
    #[inline]
    #[allow(clippy::too_many_arguments)]
    pub fn new(location_id: impl Into<String>, creds_path: impl Into<PathBuf>, certs_path: impl Into<PathBuf>, packages_path: impl Into<PathBuf>, data_path: impl Into<PathBuf>, results_path: impl Into<PathBuf>, temp_data_path: impl Into<PathBuf>, temp_results_path: impl Into<PathBuf>, checker_endpoint: impl Into<String>, proxy_address: Option<impl Into<String>>, keep_container: bool) -> Self {
        Self {
            location_id : location_id.into(),

            creds_path        : creds_path.into(),
            certs_path        : certs_path.into(),
            packages_path     : packages_path.into(),
            data_path         : data_path.into(),
            results_path      : results_path.into(),
            temp_data_path    : temp_data_path.into(),
            temp_results_path : temp_results_path.into(),

            checker_endpoint : checker_endpoint.into(),
            proxy_address    : proxy_address.map(|p| p.into()),
            keep_container,
        }
    }
}

/// Helper structure for grouping together task-dependent "constants", but that are not part of the task itself.
#[derive(Clone, Debug)]
pub struct ControlNodeInfo {
    /// The address of the API service.
    pub api_endpoint : String,
}
impl ControlNodeInfo {
    /// Constructor for the ControlNodeInfo.
    /// 
    /// # Arguments
    /// - `api_endpoint`: The address of the API service.
    /// 
    /// # Returns
    /// A new ControlNodeInfo instance.
    #[inline]
    pub fn new(api_endpoint: impl Into<String>) -> Self {
        Self {
            api_endpoint : api_endpoint.into(),
        }
    }
}

/// Helper structure for grouping together task information.
#[derive(Clone, Debug)]
pub struct TaskInfo {
    /// The name of the task to execute.
    pub name : String,

    /// The name of the task's parent package.
    pub package_name    : String,
    /// The version of the task's parent package.
    pub package_version : Version,
    /// The kind of the task to execute.
    pub kind            : Option<PackageKind>,
    /// The image name of the package where the task is from. Note: won't be populated until later.
    pub image           : Option<Image>,

    /// The input datasets/results to this task, if any.
    pub input  : HashMap<DataName, AccessKind>,
    /// If this call returns an intermediate result, its name is defined here.
    pub result : Option<String>,

    /// The input arguments to the task. Still need to be resolved before running.
    pub args: HashMap<String, FullValue>
}
impl TaskInfo {
    /// Constructor for the TaskInfo.
    /// 
    /// # Arguments
    /// - `name`: The name of the task to execute.
    /// - `package_name`: The name of the task's parent package.
    /// - `package_version`: The version of the task's parent package.
    /// - `input`: The input datasets/results to this task, if any.
    /// - `result`: If this call returns an intermediate result, its name is defined here.
    /// - `args`: The input arguments to the task. Still need to be resolved before running.
    /// 
    /// # Returns
    /// A new TaskInfo instance.
    #[inline]
    pub fn new(name: impl Into<String>, package_name: impl Into<String>, package_version: impl Into<Version>, input: HashMap<DataName, AccessKind>, result: Option<String>, args: HashMap<String, FullValue>) -> Self {
        Self {
            name : name.into(),

            package_name    : package_name.into(),
            package_version : package_version.into(),
            kind            : None,
            image           : None,

            input,
            result,

            args,
        }
    }
}





/***** HELPER FUNCTIONS *****/
/// Updates the client with a status update.
/// 
/// # Arguments
/// - `tx`: The channel to update the client on.
/// - `status`: The status to update the client with.
/// 
/// # Errors
/// This function may error if we failed to update the client.
pub async fn update_client(tx: &Sender<Result<TaskReply, Status>>, status: JobStatus) -> Result<(), ExecuteError> {
    // Convert the JobStatus into a code and (possible) value
    let (status, value): (TaskStatus, Option<String>) = status.into();

    // Put that in an ExecuteReply
    let reply: TaskReply = TaskReply {
        status : status as i32,
        value,
    };

    // Send it over the wire
    debug!("Updating client on '{:?}'...", status);
    if let Err(err) = tx.send(Ok(reply)).await {
        return Err(ExecuteError::ClientUpdateError{ status, err });
    }

    // Done
    Ok(())
}





/***** PLANNING FUNCTIONS *****/
/// Function that preprocesses by downloading the given tar and extracting it.
/// 
/// # Arguments
/// - `certs_path`: Path to the directory with all certificates.
/// - `temp_data_path`: The path to the folder where can store temporarily downloaded datasets.
/// - `temp_results_path`: The path to the folder where can store temporarily downloaded intermediate results.
/// - `proxy_addr`: If not None, then this is the address through which we will proxy the transfer.
/// - `location`: The location to download the tarball from.
/// - `address`: The address to download the tarball from.
/// - `data_name`: The type of the data (i.e., Data or IntermediateResult) combined with its identifier.
/// 
/// # Returns
/// The AccessKind to access the extracted data.
/// 
/// # Errors
/// This function can error for literally a million reasons - but they mostly relate to IO (file access, request success etc).
pub async fn preprocess_transfer_tar(certs_path: impl AsRef<Path>, temp_data_path: impl AsRef<Path>, temp_results_path: impl AsRef<Path>, proxy_addr: &Option<String>, location: Location, address: impl AsRef<str>, data_name: DataName) -> Result<AccessKind, PreprocessError> {
    debug!("Preprocessing by executing a data transfer");
    let certs_path        : &Path = certs_path.as_ref();
    let temp_data_path    : &Path = temp_data_path.as_ref();
    let temp_results_path : &Path = temp_results_path.as_ref();
    let address           : &str  = address.as_ref();
    debug!("Downloading from {} ({})", location, address);



    debug!("Loading certificate for location '{}'...", location);
    let (identity, ca_cert): (Identity, Certificate) = {
        // Compute the paths
        let cert_dir : PathBuf = certs_path.join(&location);
        let idfile   : PathBuf = cert_dir.join("client-id.pem");
        let cafile   : PathBuf = cert_dir.join("ca.pem");

        // Load the keypair for this location as an Identity file (for which we just smash 'em together and hope that works)
        let ident: Identity = match tfs::read(&idfile).await {
            Ok(raw) => match Identity::from_pem(&raw) {
                Ok(identity) => identity,
                Err(err)     => { return Err(PreprocessError::IdentityFileError{ path: idfile, err }); },
            },
            Err(err) => { return Err(PreprocessError::FileReadError{ what: "client identity", path: idfile, err }); },
        };
        // let ident: Identity = match load_cert(&certfile) {
        //     Ok(certs) => match Identity::from_pem(&ident) {
        //         Ok(identity) => identity,
        //         Err(err)     => { return Err(PreprocessError::IdentityFileError{ certfile, keyfile, err }); },
        //     },
        //     Err(err) => { return Err(PreprocessError::KeypairLoadError{ err }); },
        // };

        // Load the root store for this location (also as a list of certificates)
        let root: Certificate = match tfs::read(&cafile).await {
            Ok(raw) => match Certificate::from_pem(&raw) {
                Ok(root) => root,
                Err(err) => { return Err(PreprocessError::CertificateError{ path: cafile, err }); },
            },
            Err(err) => { return Err(PreprocessError::FileReadError{ what: "server cert root", path: cafile, err }); },
        };
        // let root: ReqwestCertificate = match load_cert(&cafile) {
        //     Ok(mut root) => if !root.is_empty() {
        //         match ReqwestCertificate::from_der(&root.swap_remove(0).0) {
        //             Ok(root) => root,
        //             Err(err) => { return Err(PreprocessError::RootError{ cafile, err }); },
        //         }
        //     } else {
        //         return Err(PreprocessError::EmptyCertFile{ path: cert_dir.join("ca.pem") });
        //     },
        //     Err(err) => { return Err(PreprocessError::StoreLoadError{ err }); },  
        // };

        // Return them, with the cert and key as identity
        (ident, root)
    };



    // Prepare the folder where we will download the data to
    debug!("Preparing filesystem...");
    let tar_path  : PathBuf = PathBuf::from("/tmp/tars");
    if !tar_path.is_dir() {
        if tar_path.exists() {
            return Err(PreprocessError::DirNotADirError{ what: "temporary tarball", path: tar_path });
        }
        if let Err(err) = tfs::create_dir_all(&tar_path).await {
            return Err(PreprocessError::DirCreateError{ what: "temporary tarball", path: tar_path, err });
        }
    }

    // Make sure the data folder is there
    if temp_data_path.exists() && !temp_data_path.is_dir() {
        return Err(PreprocessError::DirNotADirError{ what: "temporary data", path: temp_data_path.into() });
    } else if !temp_data_path.exists() {
        return Err(PreprocessError::DirNotExistsError{ what: "temporary data", path: temp_data_path.into() })
    }

    // Also make sure the results folder is there
    if temp_results_path.exists() && !temp_results_path.is_dir() {
        return Err(PreprocessError::DirNotADirError{ what: "temporary results", path: temp_results_path.into() });
    } else if !temp_results_path.exists() {
        return Err(PreprocessError::DirNotExistsError{ what: "temporary results", path: temp_results_path.into() })
    }

    // Also compute the final file path
    let (tar_path, data_path): (PathBuf, PathBuf) = match &data_name {
        DataName::Data(name) => {
            // Make sure the data path exists but is clean
            let data_path : PathBuf = temp_data_path.join(name);
            if data_path.exists() {
                if !data_path.is_dir() { return Err(PreprocessError::DirNotADirError{ what: "temporary data", path: data_path }); }
                if let Err(err) = tfs::remove_dir_all(&data_path).await { return Err(PreprocessError::DirRemoveError{ what: "temporary data", path: data_path, err }); }
            }

            // Create a fresh one
            debug!("Creating temporary data folder '{}'...", data_path.display());
            if let Err(err) = tfs::create_dir_all(&data_path).await {
                return Err(PreprocessError::DirCreateError{ what: "temporary data", path: data_path, err });
            }

            // Add the name of the file as the final result path
            (tar_path.join(format!("data_{}.tar.gz", name)), data_path)
        },

        DataName::IntermediateResult(name) => {
            // Make sure the result path exists
            let res_path : PathBuf = temp_results_path.join(name);
            if res_path.exists() {
                if !res_path.is_dir() { return Err(PreprocessError::DirNotADirError{ what: "temporary result", path: res_path }); }
                if let Err(err) = tfs::remove_dir_all(&res_path).await { return Err(PreprocessError::DirRemoveError{ what: "temporary result", path: res_path, err }); }
            }

            // Add the name of the file as the final result path
            (tar_path.join(format!("res_{}.tar.gz", name)), res_path)
        },
    };



    // Build the client
    debug!("Sending download request...");
    let mut client: ClientBuilder = Client::builder()
        .use_rustls_tls()
        .add_root_certificate(ca_cert)
        .identity(identity);
    if let Some(proxy_addr) = proxy_addr {
        client = client.proxy(match Proxy::all(proxy_addr) {
            Ok(proxy) => proxy,
            Err(err)  => { return Err(PreprocessError::ProxyCreateError { address: proxy_addr.into(), err }) },
        });
    }
    let client: Client = match client.build() {
        Ok(client) => client,
        Err(err)   => { return Err(PreprocessError::ClientCreateError{ err }); },
    };

    // Send a reqwest
    let res = match client.get(address).send().await {
        Ok(res)  => res,
        Err(err) => { return Err(PreprocessError::DownloadRequestError{ address: address.into(), err }); },
    };
    if !res.status().is_success() {
        return Err(PreprocessError::DownloadRequestFailure { address: address.into(), code: res.status(), message: res.text().await.ok() });
    }



    // With the request success, download it in parts
    debug!("Downloading file to '{}'...", tar_path.display());
    {
        let mut handle: tfs::File = match tfs::File::create(&tar_path).await {
            Ok(handle) => handle,
            Err(err)   => { return Err(PreprocessError::TarCreateError { path: tar_path, err }); },
        };
        let mut stream = res.bytes_stream();
        while let Some(chunk) = stream.next().await {
            // Unwrap the chunk
            let mut chunk: Bytes = match chunk {
                Ok(chunk) => chunk,
                Err(err)  => { return Err(PreprocessError::DownloadStreamError { address: address.into(), err }); },  
            };

            // Write it to the file
            if let Err(err) = handle.write_all_buf(&mut chunk).await {
                return Err(PreprocessError::TarWriteError{ path: tar_path, err });
            }
        }
    }



    // It took a while, but we now have the tar file; extract it
    debug!("Unpacking '{}' to '{}'...", tar_path.display(), data_path.display());
    if let Err(err) = unarchive_async(tar_path, &data_path).await {
        return Err(PreprocessError::DataExtractError{ err });
    }



    // Done; send back the reply
    Ok(AccessKind::File{ path: data_path })
}





/***** EXECUTION FUNCTIONS *****/
/// Runs the given workflow by the checker to see if it's authorized.
/// 
/// # Arguments
/// - `endpoint`: The address where the checker may be found.
/// - `workflow`: The workflow to check.
/// - `container_hash`: The hash of the container that we may use to identify it.
/// 
/// # Returns
/// Whether the workflow has been accepted or not.
/// 
/// # Errors
/// This function errors if we failed to reach the checker, or the checker itself crashed.
async fn assert_workflow_permission(endpoint: impl AsRef<str>, _workflow: &Workflow, container_hash: impl AsRef<str>) -> Result<bool, AuthorizeError> {
    let _endpoint      : &str = endpoint.as_ref();
    let container_hash : &str = container_hash.as_ref();

    // // Prepare the input struct
    // let body: CheckerRequestBody<&Workflow> = CheckerRequestBody {
    //     token : "abc".into(),
    //     workflow,
    // };

    // // Send it as a request to the client
    // let client: reqwest::Client = match reqwest::Client::builder().build() {
    //     Ok(client) => client,
    //     Err(err)   => { return Err(AuthorizeError::ClientError{ err }); },
    // };
    // let req: reqwest::Request = match client.request(reqwest::Method::POST, format!("{}", endpoint))
    //     .json(&body)
    //     .build()
    // {
    //     Ok(req)  => req,
    //     Err(err) => { return Err(AuthorizeError::RequestError{ endpoint: format!("{}", endpoint), err }); }  ,
    // };
    // let res: reqwest::Response = match client.execute(req).await {
    //     Ok(res)  => res,
    //     Err(err) => { return Err(AuthorizeError::SendError{ endpoint: format!("{}", endpoint), err }); },
    // };

    // // Match on the status code
    // let allowed: bool = match res.status() {
    //     reqwest::StatusCode::OK        => true,
    //     reqwest::StatusCode::FORBIDDEN => false,
    //     code                           => { return Err(AuthorizeError::RequestFailed{ endpoint: format!("{}", endpoint), code, body: res.text().await.unwrap_or(String::from("???")) }); },
    // };

    // Due to time constraints, we have to use some hardcoded policies :(
    // (man would I have liked to integrate eFLINT into this)

    // Allow it if it's the hash of Rosanne's container
    let rosanne_hash: &str = "feRPdYmDhwnhmMoGDcJ9yI38jzMaPdHIm4i0lTVjD2s=";
    debug!("Asserting if container hash '{}' equals Rosanne's container hash '{}'...", container_hash, rosanne_hash);
    if container_hash == rosanne_hash { return Ok(true) }

    // OOtherwise, not allowed
    Ok(false)
}



/// Downloads a container to the local registry.
/// 
/// # Arguments
/// - `einfo`: Defines information about the worker's environment. In this case, defines where to download packages to and if we should proxy the request.
/// - `endpoint`: The address where to download the container from.
/// - `image`: The image name (including digest, for caching) to download.
/// 
/// # Returns
/// The path of the downloaded image file combined with the hash of the image. It's very good practise to use this one, since the actual path is subject to change.
/// 
/// The given Image is also updated with any new digests if none are given.
/// 
/// # Errors
/// This function may error if we failed to reach the remote host, download the file or write the file.
async fn download_container(einfo: &EnvironmentInfo, endpoint: impl AsRef<str>, image: &mut Image) -> Result<(PathBuf, String), ExecuteError> {
    let endpoint: &str = endpoint.as_ref();
    debug!("Downloading image '{}' from '{}'...", image, endpoint);

    // Check if we have already downloaded it, by any chance
    let image_path : PathBuf = einfo.packages_path.join(format!("{}-{}.tar", image.name, image.version.as_ref().unwrap_or(&"latest".into())));
    let hash_path  : PathBuf = einfo.packages_path.join(format!("{}-{}.sha256", image.name, image.version.as_ref().unwrap_or(&"latest".into())));
    if image_path.exists() {
        debug!("Image file '{}' already exists; checking if it's up-to-date...", image_path.display());

        // Get the digest of the local image
        let image_digest: String = match docker::get_digest(&image_path).await {
            Ok(digest) => digest,
            Err(err)   => { return Err(ExecuteError::DigestError{ path: image_path, err }); },
        };

        // Compare the digests if they've given us one as well
        match &image.digest {
            Some(digest) => {
                if digest == &image_digest {
                    debug!("Local image is up-to-date");

                    debug!("Loading hash...");
                    let hash: String = match tfs::read_to_string(&hash_path).await {
                        Ok(hash) => hash,
                        Err(err) => { return Err(ExecuteError::HashReadError{ path: hash_path, err }); },
                    };

                    // Return both of them
                    return Ok((image_path, hash));
                }
            },
            None => {
                warn!("No digest given in request; assuming local image is out-of-date");
                image.digest = Some(image_digest);
            },
        };

        // Otherwise, they don't compare
        debug!("Local image is outdated; overwriting...");
    }

    // Setup the reqwest client
    let mut builder: ClientBuilder = Client::builder();
    if let Some(proxy) = &einfo.proxy_address {
        builder = builder.proxy(match Proxy::all(proxy) {
            Ok(proxy) => proxy,
            Err(err)  => { return Err(ExecuteError::ProxyCreateError { address: proxy.into(), err }) },
        });
    }
    let client: Client = match builder.build() {
        Ok(client) => client,
        Err(err)   => { return Err(ExecuteError::ClientCreateError{ err }); },
    };

    // Send a GET-request to the correct location
    let address: String = format!("{}/packages/{}/{}", endpoint, image.name, image.version.as_ref().unwrap_or(&"latest".into()));
    debug!("Performing request to '{}'...", address);
    let res = match client.get(&address).send().await {
        Ok(res)  => res,
        Err(err) => { return Err(ExecuteError::DownloadRequestError{ address, err }); },
    };
    if !res.status().is_success() {
        return Err(ExecuteError::DownloadRequestFailure{ address, code: res.status(), message: res.text().await.ok() });
    }

    // With the request success, download it in parts
    debug!("Writing request stream to '{}'...", image_path.display());
    {
        let mut handle: tfs::File = match tfs::File::create(&image_path).await {
            Ok(handle) => handle,
            Err(err)   => { return Err(ExecuteError::ImageCreateError{ path: image_path, err }); },
        };
        let mut stream = res.bytes_stream();
        while let Some(chunk) = stream.next().await {
            // Unwrap the chunk
            let mut chunk: Bytes = match chunk {
                Ok(chunk) => chunk,
                Err(err)  => { return Err(ExecuteError::DownloadStreamError{ address, err }); },  
            };

            // Write it to the file
            if let Err(err) = handle.write_all_buf(&mut chunk).await {
                return Err(ExecuteError::ImageWriteError{ path: image_path, err });
            }
        }
    }

    // Hash the image while at it
    debug!("Hashing image (this might take a while)...");
    let hash: String = {
        // Get the image hash
        let hash: String = hash_container(&image_path).await?;

        // Write it
        if let Err(err) = tfs::write(&hash_path, hash.as_bytes()).await {
            return Err(ExecuteError::HashWriteError{ path: hash_path, err });
        }

        // Done, return the hash
        hash
    };

    // That's OK - now return
    Ok((image_path, hash))
}

/// Given an already downloaded container, computes the SHA-256 hash of it.
/// 
/// # Arguments
/// - `container_path`: The path to the container image file to hash.
/// 
/// # Returns
/// The hash, as a `sha2::Digest`.
/// 
/// # Errors
/// This function may error if we failed to read the given file.
async fn hash_container(container_path: impl AsRef<Path>) -> Result<String, ExecuteError> {
    let container_path: &Path = container_path.as_ref();
    debug!("Hashing image file '{}'...", container_path.display());

    // Attempt to open the file
    let mut handle: tfs::File = match tfs::File::open(container_path).await {
        Ok(handle) => handle,
        Err(err)   => { return Err(ExecuteError::ImageOpenError{ path: container_path.into(), err }); },
    };

    // Read through it in chunks
    let mut hasher : Sha256 = Sha256::new();
    let mut buf    : [u8; 1024 * 16] = [0; 1024 * 16];
    loop {
        // Read the next chunk
        let n_bytes: usize = match handle.read(&mut buf).await {
            Ok(n_bytes) => n_bytes,
            Err(err)    => { return Err(ExecuteError::ImageReadError { path: container_path.into(), err }); },
        };
        // Stop if we read nothing
        if n_bytes == 0 { break; }

        // Hash that
        hasher.update(&buf[..n_bytes]);
    }
    let result: String = Base64::encode_string(&hasher.finalize());
    debug!("Image file '{}' hash: '{}'", container_path.display(), result);

    // Done
    Ok(result)
}



/// Runs the given task on a local backend.
/// 
/// # Arguments
/// - `socket_path`: Path to the Docker socket to connect to.
/// - `client_version`: The version of the Docker client we will use to talk to the engine.
/// - `tx`: The transmission channel over which we should update the client of our progress.
/// - `container_path`: The path of the downloaded container that we should execute.
/// - `einfo`: The EnvironmentInfo that specifies where to find domain-local folders, services, etc.
/// - `tinfo`: The TaskInfo that describes the task itself to execute.
/// 
/// # Returns
/// The return value of the task when it completes..
/// 
/// # Errors
/// This function errors if the task fails for whatever reason or we didn't even manage to launch it.
async fn execute_task_local(socket_path: impl AsRef<str>, client_version: ClientVersion, tx: &Sender<Result<TaskReply, Status>>, container_path: impl AsRef<Path>, einfo: EnvironmentInfo, tinfo: TaskInfo) -> Result<FullValue, JobStatus> {
    let socket_path    : &str     = socket_path.as_ref();
    let container_path : &Path    = container_path.as_ref();
    let mut tinfo      : TaskInfo = tinfo;
    let image          : Image    = tinfo.image.unwrap();
    debug!("Spawning container '{}' as a local container...", image);

    // First, we preprocess the arguments
    let binds: Vec<VolumeBind> = match docker::preprocess_args(&mut tinfo.args, &tinfo.input, &tinfo.result, Some(&einfo.data_path), &einfo.results_path).await {
        Ok(binds) => binds,
        Err(err)  => { return Err(JobStatus::CreationFailed(format!("Failed to preprocess arguments: {}", err))); },
    };

    // Serialize them next
    let params: String = match serde_json::to_string(&tinfo.args) {
        Ok(params) => params,
        Err(err)   => { return Err(JobStatus::CreationFailed(format!("Failed to serialize arguments: {}", err))); },
    };

    // Prepare the ExecuteInfo
    let info: ExecuteInfo = ExecuteInfo::new(
        &tinfo.name,
        image,
        Some(container_path.into()),
        vec![
            "-d".into(),
            "--application-id".into(),
            "unspecified".into(),
            "--location-id".into(),
            einfo.location_id,
            "--job-id".into(),
            "unspecified".into(),
            tinfo.kind.unwrap().into(),
            tinfo.name.clone(),
            base64::encode(params),
        ],
        binds,
        vec![],
        Network::None,
    );

    // Now we can launch the container...
    let name: String = match docker::launch(info, socket_path, client_version).await {
        Ok(name) => name,
        Err(err) => { return Err(JobStatus::CreationFailed(format!("Failed to spawn container: {}", err))); },
    };
    if let Err(err) = update_client(tx, JobStatus::Created).await { error!("{}", err); }
    if let Err(err) = update_client(tx, JobStatus::Started).await { error!("{}", err); }

    // ...and wait for it to complete
    let (code, stdout, stderr): (i32, String, String) = match docker::join(name, socket_path, client_version, einfo.keep_container).await {
        Ok(name) => name,
        Err(err) => { return Err(JobStatus::CompletionFailed(format!("Failed to join container: {}", err))); },
    };
    debug!("Container return code: {}", code);
    debug!("Container stdout/stderr:\n\nstdout:\n{}\n\nstderr:\n{}\n", BlockFormatter::new(&stdout), BlockFormatter::new(&stderr));
    if let Err(err) = update_client(tx, JobStatus::Completed).await { error!("{}", err); }

    // If the return code is no bueno, error and show stderr
    if code != 0 {
        return Err(JobStatus::Failed(code, stdout, stderr));
    }

    // Otherwise, decode the output of branelet to the value returned
    let output = stdout.lines().last().unwrap_or_default().to_string();
    let raw: String = match decode_base64(output) {
        Ok(raw)  => raw,
        Err(err) => { return Err(JobStatus::DecodingFailed(format!("Failed to decode output ase base64: {}", err))); },
    };
    let value: FullValue = match serde_json::from_str::<Option<FullValue>>(&raw) {
        Ok(value) => value.unwrap_or(FullValue::Void),
        Err(err)  => { return Err(JobStatus::DecodingFailed(format!("Failed to decode output as JSON: {}", err))); },
    };

    // Done
    debug!("Task '{}' returned value: '{:?}'", tinfo.name, value);
    Ok(value)
}



/// Runs the given task on the backend.
/// 
/// # Arguments
/// - `tx`: The channel to transmit stuff back to the client on.
/// - `einfo`: The EnvironmentInfo that specifies where to find domain-local folders, services, etc.
/// - `cinfo`: The ControlNodeInfo that specifies where to find services over at the control node.
/// - `workflow`: The Workflow that we're executing. Useful for communicating with the eFLINT backend.
/// - `tinfo`: The TaskInfo that describes the task itself to execute.
/// 
/// # Returns
/// Nothing directly, although it does communicate updates, results and errors back to the client via the given `tx`.
/// 
/// # Errors
/// This fnction may error for many many reasons, but chief among those are unavailable backends or a crashing task.
async fn execute_task(tx: Sender<Result<TaskReply, Status>>, einfo: EnvironmentInfo, cinfo: ControlNodeInfo, workflow: Workflow, tinfo: TaskInfo) -> Result<(), ExecuteError> {
    let mut tinfo = tinfo;

    // We update the user first on that the job has been received
    info!("Starting execution of task '{}'", tinfo.name);
    if let Err(err) = update_client(&tx, JobStatus::Received).await { error!("{}", err); }



    /* CALL PREPARATION */
    // Next, query the API for a package index
    let index: PackageIndex = match get_package_index(&format!("{}/graphql", cinfo.api_endpoint)).await {
        Ok(index) => index,
        Err(err)  => { return err!(tx, ExecuteError::PackageIndexError{ endpoint: cinfo.api_endpoint.clone(), err }); },
    };

    // Get the info
    let info: &PackageInfo = match index.get(&tinfo.package_name, Some(&tinfo.package_version)) {
        Some(info) => info,
        None       => { return err!(tx, ExecuteError::UnknownPackage{ name: tinfo.package_name.clone(), version: tinfo.package_version.clone() }); },
    };

    // Deduce the image name from that
    tinfo.kind  = Some(info.kind);
    tinfo.image = Some(Image::new(&tinfo.package_name, Some(tinfo.package_version.clone()), info.digest.clone()));

    // Now load the credentials file to get things going
    let creds: CredsFile = match CredsFile::from_path(&einfo.creds_path) {
        Ok(creds) => creds,
        Err(err)  => { return err!(tx, ExecuteError::CredsFileError{ path: einfo.creds_path.clone(), err }); },
    };

    // Download the container from the central node
    let (container_path, container_hash): (PathBuf, String) = download_container(&einfo, &cinfo.api_endpoint, tinfo.image.as_mut().unwrap()).await?;



    /* AUTHORIZATION */
    // First: make sure that the workflow is allowed by the checker
    match assert_workflow_permission(&einfo.checker_endpoint, &workflow, container_hash).await {
        Ok(true) => {
            debug!("Checker accepted incoming workflow");
            if let Err(err) = update_client(&tx, JobStatus::Authorized).await { error!("{}", err); }
        },
        Ok(false) => {
            debug!("Checker rejected incoming workflow");
            if let Err(err) = update_client(&tx, JobStatus::Denied).await { error!("{}", err); }
            return Err(ExecuteError::AuthorizationFailure{ checker: einfo.checker_endpoint });
        },

        Err(err) => {
            return err!(tx, JobStatus::AuthorizationFailed, ExecuteError::AuthorizationError{ checker: einfo.checker_endpoint.clone(), err });
        },
    }



    /* SCHEDULE */
    // Match on the specific type to find the specific backend
    let value: FullValue = match creds.method {
        Credentials::Local { path, version } => {
            match execute_task_local(path.unwrap_or_else(|| PathBuf::from("/var/run/docker.sock")).to_string_lossy(), version.map(|(major, minor)| ClientVersion{ major_version: major, minor_version: minor }).unwrap_or(*API_DEFAULT_VERSION), &tx, container_path, einfo, tinfo).await {
                Ok(value)   => value,
                Err(status) => {
                    error!("Job failed with status: {:?}", status);
                    if let Err(err) = update_client(&tx, status).await { error!("{}", err); }
                    return Ok(());
                },
            }
        },

        Credentials::Ssh { .. } => {
            error!("SSH backend is not yet supported");
            if let Err(err) = update_client(&tx, JobStatus::CreationFailed("SSH backend is not yet supported".into())).await { error!("{}", err); }
            return Ok(())
        },

        Credentials::Kubernetes { .. } => {
            error!("Kubernetes backend is not yet supported");
            if let Err(err) = update_client(&tx, JobStatus::CreationFailed("Kubernetes backend is not yet supported".into())).await { error!("{}", err); }
            return Ok(())
        },
        Credentials::Slurm { .. } => {
            error!("Slurm backend is not yet supported");
            if let Err(err) = update_client(&tx, JobStatus::CreationFailed("Slurm backend is not yet supported".into())).await { error!("{}", err); }
            return Ok(())
        },
    };
    debug!("Job completed");



    /* RETURN */
    // Alright, we are done; the rest is up to the little branelet itself.
    if let Err(err) = update_client(&tx, JobStatus::Finished(value)).await { error!("{}", err); }
    Ok(())
}



/// Commits the given intermediate result.
/// 
/// # Arguments
/// - `data_path`: Path to the shared data directory with the registry. We will update this directory as needed.
/// - `results_path`: Path to the shared data results directory. This is where the results live.
/// - `name`: The name of the intermediate result to promote.
/// - `data_name`: The name of the intermediate result to promote it as.
/// 
/// # Errors
/// This function may error for many many reasons, but chief among those are unavailable registries and such.
async fn commit_result(data_path: impl AsRef<Path>, results_path: impl AsRef<Path>, name: impl AsRef<str>, data_name: impl AsRef<str>) -> Result<(), CommitError> {
    let data_path    : &Path = data_path.as_ref();
    let results_path : &Path = results_path.as_ref();
    let name         : &str  = name.as_ref();
    let data_name    : &str  = data_name.as_ref();
    debug!("Commit intermediate result '{}' as '{}'...", name, data_name);



    // Step 1: Check if the dataset already exists (locally)
    let info: Option<AssetInfo> = {
        // Get the entries in the dataset directory
        let mut entries: tfs::ReadDir = match tfs::read_dir(data_path).await {
            Ok(entries) => entries,
            Err(err)    => { return Err(CommitError::DirReadError { path: data_path.into(), err }); },
        };

        // Iterate through them
        let mut found_info : Option<AssetInfo> = None;
        let mut i          : usize             = 0;
        #[allow(irrefutable_let_patterns)]
        while let entry = entries.next_entry().await {
            // Unwrap it
            let entry: tfs::DirEntry = match entry {
                Ok(Some(entry)) => entry,
                Ok(None)        => { break; },
                Err(err)        => { return Err(CommitError::DirEntryReadError{ path: data_path.into(), i, err }); },
            };

            // Match on directory or not
            let entry_path: PathBuf = entry.path();
            if entry_path.is_dir() {
                // Try to find the data.yml
                let info_path: PathBuf = entry_path.join("data.yml");
                if !info_path.exists() { warn!("Directory '{}' is in the data folder, but does not have a `data.yml` file", entry_path.display()); continue; }
                if !info_path.is_file() { warn!("Directory '{}' is in the data folder, but the nested `data.yml` file is not a file", entry_path.display()); continue; }

                // Load it
                let mut info: AssetInfo = match AssetInfo::from_path(&info_path) {
                    Ok(info) => info,
                    Err(err) => { return Err(CommitError::AssetInfoReadError{ path: info_path, err }); },
                };

                // Canonicalize the assetinfo's path
                match &mut info.access {
                    AccessKind::File { path } => {
                        if path.is_relative() {
                            *path = entry_path.join(&path);
                        }
                    }
                }

                // Keep it if it has the target name
                if info.name == data_name {
                    found_info = Some(info);
                    break;
                }
            }

            // Continue
            i += 1;
        }

        // Done, return the option
        found_info
    };



    // Step 2: Match on whether it already exists or not and copy the file
    if let Some(info) = info {
        debug!("Dataset '{}' already exists; overwriting file...", data_name);

        // Copy the source to the target destination (file, in this case)
        match &info.access {
            AccessKind::File { path: data_path } => {
                // Remove the old directory first (or file)
                if data_path.is_file() {
                    if let Err(err) = tfs::remove_file(&data_path).await {
                        return Err(CommitError::FileRemoveError{ path: data_path.clone(), err });
                    }

                } else if data_path.is_dir() {
                    if let Err(err) = tfs::remove_dir_all(&data_path).await {
                        return Err(CommitError::DirRemoveError{ path: data_path.clone(), err });
                    }

                } else if data_path.exists() {
                    return Err(CommitError::PathNotFileNotDir{ path: data_path.clone() });

                } else {
                    // Nothing to remove
                    warn!("Previous dataset '{}' is marked as existing, but its data doesn't exist", data_path.display());
                }

                // Simply copy the one directory over the other and it's updated
                if let Err(err) = copy_dir_recursively_async(results_path.join(name), data_path).await {
                    return Err(CommitError::DataCopyError{ err });
                };
            },
        }

    } else {
        debug!("Dataset '{}' doesn't exist; creating new entry...", data_name);

        // Prepare the package directory by creating it if it doesn't exist yet
        let dir : PathBuf = data_path.join(data_name);
        if !dir.is_dir() {
            if dir.exists() { return Err(CommitError::DataDirNotADir{ path: dir }); }
            if let Err(err) = tfs::create_dir_all(&dir).await { return Err(CommitError::DataDirCreateError{ path: dir, err }); }
        }

        // Copy the directory first, to not have the registry use it yet while copying
        if let Err(err) = copy_dir_recursively_async(results_path.join(name), dir.join("data")).await {
            return Err(CommitError::DataCopyError{ err });
        };

        // Create a new AssetInfo struct
        let info: AssetInfo = AssetInfo {
            name        : data_name.into(),
            owners      : None, // TODO: Merge parent datasets??
            description : None, // TODO: Add parents & algorithm in description??
            created     : Utc::now(),

            access : AccessKind::File{ path: dir.join("data") },
        };

        // Now write that
        let info_path  : PathBuf   = dir.join("data.yml");
        let mut handle : tfs::File = match tfs::File::create(&info_path).await {
            Ok(handle) => handle,
            Err(err)   => { return Err(CommitError::DataInfoCreateError{ path: info_path, err }); },
        };
        let sinfo: String = match serde_json::to_string_pretty(&info) {
            Ok(sinfo) => sinfo,
            Err(err)  => { return Err(CommitError::DataInfoSerializeError{ err }); },
        };
        if let Err(err) = handle.write_all(sinfo.as_bytes()).await {
            return Err(CommitError::DataInfoWriteError{ path: info_path, err });
        }
    }



    // Step 3: Enjoy
    Ok(())
}





/***** LIBRARY *****/
/// Defines a server for incoming worker requests.
#[derive(Debug)]
pub struct WorkerServer {
    /// The information about the local environment that we store.
    env_info : EnvironmentInfo,
}

impl WorkerServer {
    /// Constructor for the JobHandler.
    /// 
    /// # Arguments
    /// - `env_info`: The EnvironmentInfo struct with everything we need to know about the local envirresults_dironment.
    /// 
    /// # Returns
    /// A new JobHandler instance.
    #[inline]
    pub fn new(env_info: EnvironmentInfo) -> Self {
        Self {
            env_info,
        }
    }
}

#[tonic::async_trait]
impl JobService for WorkerServer {
    type ExecuteStream = ReceiverStream<Result<TaskReply, Status>>;

    async fn preprocess(&self, request: Request<PreprocessRequest>) -> Result<Response<PreprocessReply>, Status> {
        let request = request.into_inner();
        debug!("Receiving preprocess request");

        // Fetch the data kind
        let data_name: DataName = match DataKind::from_i32(request.data_kind) {
            Some(DataKind::Data)               => DataName::Data(request.data_name),
            Some(DataKind::IntermediateResult) => DataName::IntermediateResult(request.data_name),
            None                               => {
                debug!("Incoming request has invalid data kind '{}' (dropping it)", request.data_kind);
                return Err(Status::invalid_argument(format!("Unknown data kind '{}'", request.data_kind)));
            }
        };

        // Parse the preprocess kind
        match PreprocessKind::from_i32(request.kind) {
            Some(PreprocessKind::TransferRegistryTar) => {
                // The given piece of data is the address
                let (location, address): (String, String) = match request.data {
                    Some(data) => match serde_json::from_str(&data) {
                        Ok(res)  => res,
                        Err(err) => {
                            debug!("Incoming request has invalid (location, address) pair: {} (dropping it)", err);
                            return Err(Status::invalid_argument("Illegal data field for TransferRegistryTar".to_string()));
                        },
                    },
                    None => {
                        debug!("Incoming request missing data field (dropping it)");
                        return Err(Status::invalid_argument("Missing data field for TransferRegistryTar".to_string()));
                    },
                };

                // Run the function that way
                let access: AccessKind = match preprocess_transfer_tar(&self.env_info.certs_path, &self.env_info.temp_data_path, &self.env_info.temp_results_path, &self.env_info.proxy_address, location, address, data_name).await {
                    Ok(access) => access,
                    Err(err)   => {
                        error!("{}", err);
                        return Err(Status::internal("An internal error occurred"));
                    }
                };

                // Serialize the accesskind and return the reply
                let saccess: String = match serde_json::to_string(&access) {
                    Ok(saccess) => saccess,
                    Err(err)    => {
                        error!("{}", PreprocessError::AccessKindSerializeError { err });
                        return Err(Status::internal("An internal error occurred"));
                    },
                };

                // Done
                debug!("File transfer complete.");
                Ok(Response::new(PreprocessReply {
                    ok     : true,
                    access : saccess,
                }))
            },

            None => {
                debug!("Incoming request has invalid preprocess kind '{}' (dropping it)", request.kind);
                Err(Status::invalid_argument(format!("Unknown preprocesskind '{}'", request.kind)))
            },
        }
    }



    async fn execute(&self, request: Request<TaskRequest>) -> Result<Response<Self::ExecuteStream>, Status> {
        let request = request.into_inner();
        debug!("Receiving execute request");

        // Prepare gRPC stream between client and (this) job delegate.
        let (tx, rx) = mpsc::channel::<Result<TaskReply, Status>>(10);

        // Attempt to parse the workflow
        let workflow: Workflow = match serde_json::from_str(&request.workflow) {
            Ok(workflow) => workflow,
            Err(err)     => {
                error!("Failed to deserialize workflow: {}", err);
                debug!("Workflow:\n{}\n{}\n{}\n", (0..80).map(|_| '-').collect::<String>(), request.workflow, (0..80).map(|_| '-').collect::<String>());
                if let Err(err) = tx.send(Err(Status::invalid_argument(format!("Failed to deserialize workflow: {}", err)))).await { error!("{}", err); }
                return Ok(Response::new(ReceiverStream::new(rx)));
            },
        };

        // Attempt to parse the version
        let version: Version = match Version::from_str(&request.package_version) {
            Ok(version) => version,
            Err(err)    => {
                error!("Failed to deserialize version '{}': {}", request.package_version, err);
                if let Err(err) = tx.send(Err(Status::invalid_argument(format!("Failed to deserialize version '{}': {}", request.package_version, err)))).await { error!("{}", err); }
                return Ok(Response::new(ReceiverStream::new(rx)));
            },
        };

        // Attempt to parse the input
        let input: HashMap<DataName, AccessKind> = match json_to_map(&request.input) {
            Ok(input) => input,
            Err(err)  => {
                error!("Failed to deserialize input '{}': {}", request.input, err);
                if let Err(err) = tx.send(Err(Status::invalid_argument(format!("Failed to deserialize input '{}': {}", request.input, err)))).await { error!("{}", err); }
                return Ok(Response::new(ReceiverStream::new(rx)));
            },
        };

        // Attempt to parse the arguments
        let args: HashMap<String, FullValue> = match serde_json::from_str(&request.args) {
            Ok(args) => args,
            Err(err) => {
                error!("Failed to deserialize arguments '{}': {}", request.args, err);
                if let Err(err) = tx.send(Err(Status::invalid_argument(format!("Failed to deserialize arguments '{}': {}", request.args, err)))).await { error!("{}", err); }
                return Ok(Response::new(ReceiverStream::new(rx)));
            },
        };

        // Collect some request data into ControlNodeInfo's and TaskInfo's.
        let cinfo : ControlNodeInfo = ControlNodeInfo::new(request.api);
        let tinfo : TaskInfo        = TaskInfo::new(
            request.name,
            request.package_name,
            version,

            input,
            request.result,
            args,
        );

        // Now move the rest to a separate thread
        tokio::spawn(execute_task(tx, self.env_info.clone(), cinfo, workflow, tinfo));

        // Return the stream so the user can get updates
        Ok(Response::new(ReceiverStream::new(rx)))
    }



    async fn commit(&self, request: Request<CommitRequest>) -> Result<Response<CommitReply>, Status> {
        let request = request.into_inner();
        debug!("Receiving commit request");

        // Run the function
        if let Err(err) = commit_result(&self.env_info.data_path, &self.env_info.results_path, &request.name, &request.data_name).await {
            error!("{}", err);
            return Err(Status::internal("An internal error occurred"));
        }

        // Be done without any error
        Ok(Response::new(CommitReply{ ok: true, error: None }))
    }
}
