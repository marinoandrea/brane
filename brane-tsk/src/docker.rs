//  DOCKER.rs
//    by Lut99
// 
//  Created:
//    19 Sep 2022, 14:57:17
//  Last edited:
//    21 Nov 2022, 11:26:12
//  Auto updated?
//    Yes
// 
//  Description:
//!   Defines functions that interact with the local Docker daemon.
// 

use std::collections::HashMap;
use std::fmt::{Display, Formatter, Result as FResult};
use std::path::{Path, PathBuf};

use bollard::{API_DEFAULT_VERSION, ClientVersion, Docker};
use bollard::container::{
    Config, CreateContainerOptions, LogOutput, LogsOptions, RemoveContainerOptions, StartContainerOptions,
    WaitContainerOptions
};
use bollard::image::{CreateImageOptions, ImportImageOptions, RemoveImageOptions};
use bollard::models::{DeviceRequest, EndpointSettings, HostConfig};
use futures_util::stream::TryStreamExt;
use futures_util::StreamExt;
use hyper::Body;
use log::debug;
use serde::{Deserialize, Serialize};
use tokio::fs::{self as tfs, File as TFile};
use tokio::io::AsyncReadExt;
use tokio_tar::Archive;
use tokio_util::codec::{BytesCodec, FramedRead};

use brane_ast::ast::DataName;
use brane_exe::FullValue;
use brane_shr::debug::EnumDebug;
use specifications::container::{Image, VolumeBind};
use specifications::data::AccessKind;

pub use crate::errors::DockerError as Error;
use crate::errors::ExecuteError;


/***** CONSTANTS *****/
/// Defines the prefix to the Docker image tar's manifest config blob (which contains the image digest)
pub(crate) const MANIFEST_CONFIG_PREFIX: &str = "blobs/sha256/";





/***** HELPER STRUCTS *****/
/// The layout of a Docker manifest file.
#[derive(Clone, Debug, Deserialize, Serialize)]
struct DockerImageManifest {
    /// The config string that contains the digest as the path of the config file
    #[serde(rename = "Config")]
    config : String,
}





/***** AUXILLARY STRUCTS *****/
/// Defines the (type of) network ot which a container should connect.
#[derive(Clone, Debug)]
pub enum Network {
    /// Use no network.
    None,

    /// Use a bridged network (Docker's default).
    Bridge,
    /// Use the host network directly.
    Host,
    /// Connect to a specific other container (with the given name/ID).
    Container(String),
    /// Connect to a network with the given name.
    Custom(String),
}

impl Display for Network {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> FResult {
        use Network::*;
        match self {
            None => write!(f, "none"),

            Bridge          => write!(f, "bridge"),
            Host            => write!(f, "host"),
            Container(name) => write!(f, "container:{}", name),
            Custom(name)    => write!(f, "{}", name),
        }
    }
}

impl From<Network> for String {
    #[inline]
    fn from(value: Network) -> Self {
        format!("{}", value)
    }
}
impl From<&Network> for String {
    #[inline]
    fn from(value: &Network) -> Self {
        format!("{}", value)
    }
}



/// Collects information we need to perform a container call.
#[derive(Clone, Debug)]
pub struct ExecuteInfo {
    /// The name of the container-to-be.
    pub name       : String,
    /// The image name to use for the container.
    pub image      : Image,
    /// The raw image.tar file we would like to mount first if the image itself is missing. If omitted, then we're pulling the image's name as-is with the engine.
    pub image_file : Option<PathBuf>,

    /// The command(s) to pass to Branelet.
    pub command : Vec<String>,
    /// The extra mounts we want to add, if any (this includes any data folders).
    pub binds   : Vec<VolumeBind>,
    /// The extra device requests we want to add, if any (e.g., GPUs).
    pub devices : Vec<DeviceRequest>,
    /// The netwok to connect the container to.
    pub network : Network,
}

impl ExecuteInfo {
    /// Constructor for the ExecuteInfo.
    ///
    /// # Arguments
    /// - `name`: The name of the container-to-be.
    /// - `image`: The image name to use for the container.
    /// - `image_file`: The raw image.tar file we would like to mount first if the image itself is missing. If omitted, then we're pulling the image's name as-is with the engine.
    /// - `command`: The command(s) to pass to Branelet.
    /// - `binds`: The extra mounts we want to add, if any (this includes any data folders).
    /// - `devices`: The extra device requests we want to add, if any (e.g., GPUs).
    /// - `network`: The netwok to connect the container to.
    /// 
    /// # Returns
    /// A new ExecuteInfo instance populated with the given values.
    #[inline]
    pub fn new(name: impl Into<String>, image: Image, image_file: Option<PathBuf>, command: Vec<String>, binds: Vec<VolumeBind>, devices: Vec<DeviceRequest>, network: Network) -> Self {
        ExecuteInfo {
            name : name.into(),
            image,
            image_file,

            command,
            binds,
            devices,
            network,
        }
    }
}





/***** HELPER FUNCTIONS *****/
/// Preprocesses a single argument from either an IntermediateResult or a Data to whatever is needed for their access kind and any mounts.
/// 
/// # Arguments
/// - `data_dir`: The directory where all real datasets live.
/// - `results_dir`: The directory where to mount results from.
/// - `binds`: The list of VolumeBinds to which we will add while preprocessing.
/// - `inputs`: The list of inputs to resolve the name in.
/// - `name`: The name of the argument.
/// - `value`: The FullValue to preprocess.
/// 
/// # Returns
/// Nothing explicitly, but does add to the list of binds and overwrites the value of the given FullValue with any other one if necessary.
/// 
/// # Errors
/// This function errors if we didn't know the input set or if we failed to create new volume binds.
fn preprocess_arg(data_dir: Option<impl AsRef<Path>>, results_dir: impl AsRef<Path>, binds: &mut Vec<VolumeBind>, input: &HashMap<DataName, AccessKind>, name: impl AsRef<str>, value: &mut FullValue) -> Result<(), ExecuteError> {
    let data_dir    : Option<&Path> = data_dir.as_ref().map(|d| d.as_ref());
    let results_dir : &Path         = results_dir.as_ref();
    let name        : &str          = name.as_ref();

    // Match on its type to find its data name
    let data_name: DataName = match value {
        // The Data and IntermediateResult is why we're here
        FullValue::Data(name)               => DataName::Data(name.into()),
        FullValue::IntermediateResult(name) => DataName::IntermediateResult(name.into()),

        // Some types might need recursion
        FullValue::Array(values) => {
            for (i, v) in values.iter_mut().enumerate() {
                preprocess_arg(data_dir, results_dir, binds, input, format!("{}[{}]", name, i), v)?;
            }
            return Ok(());
        },
        FullValue::Instance(_, props) => {
            for (n, v) in props {
                preprocess_arg(data_dir, results_dir, binds, input, format!("{}.{}", name, n), v)?;
            }
            return Ok(());
        },

        // Otherwise, we don't have to preprocess
        _ => { return Ok(()); },
    };
    debug!("Resolving argument '{}' ({})", name, data_name.variant());

    // Get the method of access for this data type
    let access: &AccessKind = match input.get(&data_name) {
        Some(access) => access,
        None         => { return Err(ExecuteError::UnknownData{ name: data_name }); },
    };

    // Match on that to replace the value and generate a binding (possibly)
    match access {
        AccessKind::File { path } => {
            // If this is an intermediate result, patch the path with the results directory
            let src_dir: PathBuf = if data_name.is_intermediate_result() {
                results_dir.join(path)
            } else if let Some(data_dir) = data_dir {
                data_dir.join(data_name.name()).join(path)
            } else {
                path.clone()
            };

            // Generate the container path
            let dst_dir: PathBuf = PathBuf::from("/data").join(data_name.name());

            // Generate a volume bind with that
            binds.push(match VolumeBind::new_readonly(src_dir, &dst_dir) {
                Ok(bind) => bind,
                Err(err) => { return Err(ExecuteError::VolumeBindError{ err }); },
            });

            // Replace the argument
            *value = FullValue::String(dst_dir.to_string_lossy().to_string());
        },
    }

    // OK
    Ok(())
}



/// Tries to import/pull the given image if it does not exist in the local Docker instance.
/// 
/// # Arguments
/// - `docker`: An already connected local instance of Docker.
/// - `info`: The ExecuteInfo describing the image to pull.
/// 
/// # Errors
/// This function errors if it failed to ensure the image existed (i.e., import or pull failed).
async fn ensure_image(docker: &Docker, info: &ExecuteInfo) -> Result<(), Error> {
    // Abort if image is already loaded
    let simage: String = (&info.image).into();
    if docker.inspect_image(&simage).await.is_ok() {
        // The image is present, and because we specified the hash in the name, it's also for sure up-to-date
        debug!("Image already exists in Docker deamon.");
        return Ok(());
    } else {
        debug!("Image doesn't exist in Docker daemon.");
    }

    // Otherwise, import it if it is described or pull it
    if let Some(image_file) = &info.image_file {
        debug!(" > Importing file '{}'...", image_file.display());
        import_image(docker, image_file).await
    } else {
        debug!(" > Pulling image '{}'...", info.image);
        pull_image(docker, info.image.clone()).await
    }
}

/// Creates a container with the given image and starts it (non-blocking after that).
/// 
/// # Arguments
/// - `docker`: The Docker instance to use for accessing the container.
/// - `info`: The ExecuteInfo describing what to launch and how.
/// 
/// # Returns
/// The name of the container such that it can be waited on later.
/// 
/// # Errors
/// This function may error for many reasons, which usually means that the container failed to be created or started (wow!).
async fn create_and_start_container(docker: &Docker, info: &ExecuteInfo) -> Result<String, Error> {
    // Generate unique (temporary) container name
    let container_name: String = format!("{}-{}", info.name, &uuid::Uuid::new_v4().to_string()[..6]);
    let create_options = CreateContainerOptions { name: &container_name };

    // Combine the properties in the execute info into a HostConfig
    let host_config = HostConfig {
        binds           : Some(info.binds.iter().map(|b| { debug!("Binding '{}' (host) -> '{}' (container)", b.host.display(), b.container.display()); b.docker().to_string() }).collect()),
        network_mode    : Some(info.network.clone().into()),
        privileged      : Some(false),
        device_requests : Some(info.devices.clone()),
        ..Default::default()
    };

    // Create the container confic
    let create_config = Config {
        image       : Some(info.image.name()),
        cmd         : Some(info.command.clone()),
        host_config : Some(host_config),
        ..Default::default()
    };

    // Run it with that config
    debug!("Launching container with name '{}' (image: {})...", info.name, info.image.name());
    if let Err(reason) = docker.create_container(Some(create_options), create_config).await { return Err(Error::CreateContainerError{ name: info.name.clone(), image: info.image.clone(), err: reason }); }
    debug!(" > Container created");
    match docker.start_container(&container_name, None::<StartContainerOptions<String>>).await {
        Ok(_)       => {
            debug!(" > Container '{}' started", container_name);
            Ok(container_name)
        },
        Err(reason) => Err(Error::StartError{ name: info.name.clone(), image: info.image.clone(), err: reason })
    }
}

/// Waits for the given container to complete.
/// 
/// # Arguments
/// - `docker`: The Docker instance to use for accessing the container.
/// - `name`: The name of the container to wait on.
/// - `image`: The image that was run (used for debugging).
/// - `keep_container`: Whether to keep the container around after it's finished or not.
/// 
/// # Returns
/// The return code of the docker container, its stdout and its stderr (in that order).
/// 
/// # Errors
/// This function may error for many reasons, which usually means that the container is unknown or the Docker engine is unreachable.
async fn join_container(docker: &Docker, name: &str, keep_container: bool) -> Result<(i32, String, String), Error> {
    // Wait for the container to complete
    if let Err(reason) = docker.wait_container(name, None::<WaitContainerOptions<String>>).try_collect::<Vec<_>>().await {
        return Err(Error::WaitError{ name: name.into(), err: reason });
    }

    // Get stdout and stderr logs from container
    let logs_options = Some(LogsOptions::<String> {
        stdout: true,
        stderr: true,
        ..Default::default()
    });
    let log_outputs = match docker.logs(name, logs_options).try_collect::<Vec<LogOutput>>().await {
        Ok(out)     => out,
        Err(reason) => { return Err(Error::LogsError{ name: name.into(), err: reason }); }
    };

    // Collect them in one string per output channel
    let mut stderr = String::new();
    let mut stdout = String::new();
    for log_output in log_outputs {
        match log_output {
            LogOutput::StdErr { message } => stderr.push_str(String::from_utf8_lossy(&message).as_ref()),
            LogOutput::StdOut { message } => stdout.push_str(String::from_utf8_lossy(&message).as_ref()),
            _ => { continue; },
        }
    }

    // Get the container's exit status by inspecting it
    let code = returncode_container(docker, name).await?;

    // Don't leave behind any waste: remove container (but only if told to do so!)
    if !keep_container { remove_container(docker, name).await?; }

    // Return the return data of this container!
    Ok((code, stdout, stderr))
}

/// Returns the exit code of a container is (hopefully) already stopped.
/// 
/// # Arguments
/// - `docker`: The Docker instance to use for accessing the container.
/// - `name`: The container's name.
/// 
/// # Returns
/// The exit-/returncode that was returned by the container.
/// 
/// # Errors
/// This function errors if the Docker daemon could not be reached, such a container did not exist, could not be inspected or did not have a return code (yet).
async fn returncode_container(docker: &Docker, name: impl AsRef<str>) -> Result<i32, Error> {
    let name: &str = name.as_ref();

    // Do the inspect call
    let info = match docker.inspect_container(name, None).await {
        Ok(info)    => info,
        Err(reason) => { return Err(Error::InspectContainerError{ name: name.into(), err: reason }); }
    };

    // Try to get the execution state from the container
    let state = match info.state {
        Some(state) => state,
        None        => { return Err(Error::ContainerNoState{ name: name.into() }); }
    };

    // Finally, try to get the exit code itself
    match state.exit_code {
        Some(code) => Ok(code as i32),
        None       => Err(Error::ContainerNoExitCode{ name: name.into() }),
    }
}

/// Tries to remove the docker container with the given name.
/// 
/// # Arguments
/// - `docker`: An already connected local instance of Docker.
/// - `name`: The name of the container to remove.
/// 
/// # Errors
/// This function errors if we failed to remove it.
async fn remove_container(docker: &Docker, name: impl AsRef<str>) -> Result<(), Error> {
    let name: &str = name.as_ref();

    // Set the options
    let remove_options = Some(RemoveContainerOptions {
        force: true,
        ..Default::default()
    });

    // Attempt the removal
    match docker.remove_container(name, remove_options).await {
        Ok(_)       => Ok(()),
        Err(reason) => Err(Error::ContainerRemoveError{ name: name.into(), err: reason }),
    }
}

/// Tries to import the image at the given path into the given Docker instance.
/// 
/// # Arguments
/// - `docker`: An already connected local instance of Docker.
/// - `image_file`: Path to the image to import.
/// 
/// # Returns
/// Nothing on success, or an ExecutorError otherwise.
async fn import_image(docker: &Docker, image_file: impl AsRef<Path>) -> Result<(), Error> {
    let image_file : &Path = image_file.as_ref();
    let options            = ImportImageOptions { quiet: true };

    // Try to read the file
    let file = match TFile::open(image_file).await {
        Ok(handle)  => handle,
        Err(reason) => { return Err(Error::ImageFileOpenError{ path: PathBuf::from(image_file), err: reason }); }
    };

    // If successful, open the byte with a FramedReader, freezing all the chunk we read
    let byte_stream = FramedRead::new(file, BytesCodec::new()).map(|r| {
        let bytes = r.unwrap().freeze();
        Ok::<_, Error>(bytes)
    });

    // Finally, wrap it in a HTTP body and send it to the Docker API
    let body = Body::wrap_stream(byte_stream);
    match docker.import_image(options, body, None).try_collect::<Vec<_>>().await {
        Ok(_)       => Ok(()),
        Err(reason) => Err(Error::ImageImportError{ path: PathBuf::from(image_file), err: reason })
    }
}

/// Pulls a new image from the given Docker image ID / URL (?) and imports it in the Docker instance.
/// 
/// # Arguments
/// - `docker`: An already connected local instance of Docker.
/// - `image`: The image to pull.
/// 
/// # Errors
/// This function errors if we failed to pull the image, e.g., the Docker engine did not know where to find it, or there was no internet.
async fn pull_image(docker: &Docker, image: Image) -> Result<(), Error> {
    // Define the options for this image
    let options = Some(CreateImageOptions {
        from_image : image.name(),
        ..Default::default()
    });

    // Try to create it
    match docker.create_image(options, None, None).try_collect::<Vec<_>>().await {
        Ok(_)    => Ok(()),
        Err(err) => Err(Error::ImagePullError{ image, err }),
    }
}





/***** AUXILLARY FUNCTIONS *****/
/// Helps any VM aiming to use Docker by preprocessing the given list of arguments and function result into a list of bindings (and resolving the the arguments while at it).
/// 
/// # Arguments
/// - `args`: The arguments to resolve / generate bindings for.
/// - `input`: A list of input datasets & intermediate results to the current task.
/// - `result`: The result to also generate a binding for if it is present.
/// - `data_dir`: The directory where all real datasets live.
/// - `results_dir`: The directory where all temporary results are/will be stored.
/// 
/// # Returns
/// A list of VolumeBindings that define which folders have to be mounted to the container how.
/// 
/// # Errors
/// This function errors if datasets / results are unknown to us.
pub async fn preprocess_args(args: &mut HashMap<String, FullValue>, input: &HashMap<DataName, AccessKind>, result: &Option<String>, data_dir: Option<impl AsRef<Path>>, results_dir: impl AsRef<Path>) -> Result<Vec<VolumeBind>, ExecuteError> {
    let data_dir    : Option<&Path> = data_dir.as_ref().map(|r| r.as_ref());
    let results_dir : &Path         = results_dir.as_ref();

    // Then, we resolve the input datasets using the runtime index
    let mut binds: Vec<VolumeBind> = vec![];
    for (name, value) in args {
        preprocess_arg(data_dir, results_dir, &mut binds, input, name, value)?;
    }

    // Also make sure the result directory is alive and kicking
    if let Some(result) = result {
        // The source path will be `<results folder>/<name>`
        let src_path: PathBuf = results_dir.join(result);
        // The container-relevant path will be: `/result` (nice and easy)
        let ref_path: PathBuf = PathBuf::from("/result");

        // Now make sure the source path exists and is a new, empty directory
        if src_path.exists() {
            if !src_path.is_dir() { return Err(ExecuteError::ResultDirNotADir{ path: src_path }); }
            if let Err(err) = tfs::remove_dir_all(&src_path).await { return Err(ExecuteError::ResultDirRemoveError { path: src_path, err }); }
        }
        if let Err(err) = tfs::create_dir_all(&src_path).await {
            return Err(ExecuteError::ResultDirCreateError{ path: src_path, err });
        }

        // Add a volume bind for that
        binds.push(match VolumeBind::new_readwrite(src_path, ref_path) {
            Ok(bind) => bind,
            Err(err) => { return Err(ExecuteError::VolumeBindError{ err }); }
        });
    }

    // Done, return the binds
    Ok(binds)
}

/// Given an `image.tar` file, extracts the Docker digest (i.e., image ID) from it and returns it.
/// 
/// # Arguments
/// - `path`: The `image.tar` file to extract the digest from.
/// 
/// # Returns
/// The image's digest as a string. Does not include `sha:...`.
/// 
/// # Errors
/// This function errors if the given image.tar could not be read or was in an incorrect format.
pub async fn get_digest(path: impl AsRef<Path>) -> Result<String, Error> {
    // Convert the Path-like to a Path
    let path: &Path = path.as_ref();

    // Try to open the given file
    let handle: TFile = match TFile::open(path).await {
        Ok(handle) => handle,
        Err(err)   => { return Err(Error::ImageTarOpenError{ path: path.to_path_buf(), err }); }
    };

    // Wrap it as an Archive
    let mut archive: Archive<TFile> = Archive::new(handle);

    // Go through the entries
    let mut entries = match archive.entries() {
        Ok(handle) => handle,
        Err(err)   => { return Err(Error::ImageTarEntriesError{ path: path.to_path_buf(), err }); }
    };
    while let Some(entry) = entries.next().await {
        // Make sure the entry is legible
        let mut entry = match entry {
            Ok(entry) => entry,
            Err(err)  => { return Err(Error::ImageTarEntryError{ path: path.to_path_buf(), err }); }
        };

        // Check if the entry is the manifest.json
        let entry_path = match entry.path() {
            Ok(path) => path.to_path_buf(),
            Err(err) => { return Err(Error::ImageTarIllegalPath{ path: path.to_path_buf(), err }); }
        };
        if entry_path == PathBuf::from("manifest.json") {
            // Try to read it
            let mut manifest: Vec<u8> = vec![];
            if let Err(err) = entry.read_to_end(&mut manifest).await {
                return Err(Error::ImageTarManifestReadError{ path: path.to_path_buf(), entry: entry_path, err });
            };

            // Try to parse it with serde
            let mut manifest: Vec<DockerImageManifest> = match serde_json::from_slice(&manifest) {
                Ok(manifest) => manifest,
                Err(err)     => { return Err(Error::ImageTarManifestParseError{ path: path.to_path_buf(), entry: entry_path, err }); }
            };

            // Get the first and only entry from the vector
            let manifest: DockerImageManifest = if manifest.len() == 1 {
                manifest.pop().unwrap()
            } else {
                return Err(Error::ImageTarIllegalManifestNum{ path: path.to_path_buf(), entry: entry_path, got: manifest.len() });
            };

            // Now, try to strip the filesystem part and add sha256:
            let digest = if manifest.config.starts_with(MANIFEST_CONFIG_PREFIX) {
                let mut digest = String::from("sha256:");
                digest.push_str(&manifest.config[MANIFEST_CONFIG_PREFIX.len()..]);
                digest
            } else {
                return Err(Error::ImageTarIllegalDigest{ path: path.to_path_buf(), entry: entry_path, digest: manifest.config });
            };

            // We found the digest! Set it, then return
            return Ok(digest);
        }
    }

    // No manifest found :(
    Err(Error::ImageTarNoManifest{ path: path.to_path_buf() })
}





/***** LIBRARY *****/
/// Launches the given job and returns its name so it can be tracked.
/// 
/// Note that this function makes its own connection to the local Docker daemon.
///
/// # Arguments
/// - `exec`: The ExecuteInfo that describes the job to launch.
/// - `path`: The path to the Docker socket to connect to.
/// - `version`: The version of the client we use to connect to the daemon.
/// 
/// # Returns
/// The name of the container such that it can be waited on later.
/// 
/// # Errors
/// This function errors for many reasons, some of which include not being able to connect to Docker or the container failing (to start).
pub async fn launch(exec: ExecuteInfo, path: impl AsRef<Path>, version: ClientVersion) -> Result<String, Error> {
    let path: &Path = path.as_ref();

    // Connect to docker
    let docker = match Docker::connect_with_unix(&path.to_string_lossy(), 120, &version) {
        Ok(res)     => res,
        Err(reason) => { return Err(Error::ConnectionError{ path: path.into(), version, err: reason }); }
    };

    // Either import or pull image, if not already present
    ensure_image(&docker, &exec).await?;

    // Start container, return immediately (propagating any errors that occurred)
    create_and_start_container(&docker, &exec).await
}

/// Joins the container with the given name, i.e., waits for it to complete and returns its results.
/// 
/// # Arguments
/// - `name`: The name of the container to wait for.
/// - `path`: The path to the Docker socket to connect to.
/// - `version`: The version of the client we use to connect to the daemon.
/// - `keep_container`: If true, then will not remove the container after it has been launched. This is very useful for debugging.
/// 
/// # Returns
/// The return code of the docker container, its stdout and its stderr (in that order).
/// 
/// # Errors
/// This function may error for many reasons, which usually means that the container is unknown or the Docker engine is unreachable.
pub async fn join(name: impl AsRef<str>, path: impl AsRef<Path>, version: ClientVersion, keep_container: bool) -> Result<(i32, String, String), Error> {
    let name : &str  = name.as_ref();
    let path : &Path = path.as_ref();

    // Connect to docker
    let docker = match Docker::connect_with_unix(&path.to_string_lossy(), 120, &version) {
        Ok(res)     => res,
        Err(reason) => { return Err(Error::ConnectionError{ path: path.into(), version, err: reason }); }
    };

    // And now wait for it
    join_container(&docker, name, keep_container).await
}

/// Launches the given container and waits until its completed.
/// 
/// Note that this function makes its own connection to the local Docker daemon.
///
/// # Arguments
/// - `exec`: The ExecuteInfo describing what to launch and how.
/// - `keep_container`: If true, then will not remove the container after it has been launched. This is very useful for debugging.
/// 
/// # Returns
/// The return code of the docker container, its stdout and its stderr (in that order).
/// 
/// # Errors
/// This function errors for many reasons, some of which include not being able to connect to Docker or the container failing.
pub async fn run_and_wait(exec: ExecuteInfo, keep_container: bool) -> Result<(i32, String, String), Error> {
    // This next bit's basically launch but copied so that we have a docker connection of our own.
    // Connect to docker
    let docker = match Docker::connect_with_local_defaults() {
        Ok(res)     => res,
        Err(reason) => { return Err(Error::ConnectionError{ path: "/var/run/docker.sock".into(), version: *API_DEFAULT_VERSION, err: reason }); }
    };

    // Either import or pull image, if not already present
    ensure_image(&docker, &exec).await?;

    // Start container, return immediately (propagating any errors that occurred)
    let name: String = create_and_start_container(&docker, &exec).await?;

    // And now wait for it
    join_container(&docker, &name, keep_container).await
}

/// Tries to return the (IP-)address of the container with the given name.
/// 
/// Note that this function makes a separate connection to the local Docker instance.
/// 
/// # Arguments
/// - `name`: The name of the container to fetch the address of.
/// 
/// # Returns
/// The address of the container as a string on success, or an ExecutorError otherwise.
pub async fn get_container_address(name: impl AsRef<str>) -> Result<String, Error> {
    let name: &str = name.as_ref();

    // Try to connect to the local instance
    let docker = match Docker::connect_with_local_defaults() {
        Ok(conn)    => conn,
        Err(reason) => { return Err(Error::ConnectionError{ path: "/var/run/docker.sock".into(), version: *API_DEFAULT_VERSION, err: reason }); }
    };

    // Try to inspect the container in question
    let container = match docker.inspect_container(name.as_ref(), None).await {
        Ok(data)    => data,
        Err(reason) => { return Err(Error::InspectContainerError{ name: name.into(), err: reason }); }
    };

    // Get the networks of this container
    let networks: HashMap<String, EndpointSettings> = container
        .network_settings
        .and_then(|n| n.networks)
        .unwrap_or_default();

    // Next, get the address of the first network and try to return that
    if let Some(network) = networks.values().next() {
        let ip = network.ip_address.clone().unwrap_or_default();
        if ip.is_empty() {
            Ok(String::from("127.0.0.1"))
        } else {
            Ok(ip)
        }
    } else {
        Err(Error::ContainerNoNetwork{ name: name.into() })
    }
}

/// Tries to remove the docker image with the given name.
/// 
/// Note that this function makes a separate connection to the local Docker instance.
/// 
/// # Arguments
/// - `name`: The name of the image to remove.
/// 
/// # Errors
/// This function errors if removing the image failed. Reasons for this may be if the image did not exist, the Docker engine was not reachable, or ...
pub async fn remove_image(image: &Image) -> Result<(), Error> {
    // Try to connect to the local instance
    let docker = match Docker::connect_with_local_defaults() {
        Ok(conn)    => conn,
        Err(reason) => { return Err(Error::ConnectionError{ path: "/var/run/docker.sock".into(), version: *API_DEFAULT_VERSION, err: reason }); }
    };

    // Check if the image still exists
    let info = docker.inspect_image(&image.name()).await;
    if info.is_err() {
        // It doesn't (or we can't reach it), but either way, easy
        return Ok(());
    }

    // Set the options to remove
    let remove_options = Some(RemoveImageOptions {
        force: true,
        ..Default::default()
    });

    // Now we can try to remove the image
    let info = info.unwrap();
    match docker.remove_image(&info.id, remove_options, None).await {
        Ok(_)       => Ok(()),
        Err(reason) => Err(Error::ImageRemoveError{ image: image.clone(), id: info.id.clone(), err: reason }),
    }
}
