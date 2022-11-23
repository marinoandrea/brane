 //  LIFETIME.rs
//    by Lut99
// 
//  Created:
//    22 Nov 2022, 11:19:22
//  Last edited:
//    23 Nov 2022, 17:47:41
//  Auto updated?
//    Yes
// 
//  Description:
//!   Commands that relate to managing the lifetime of the local node.
// 

use std::collections::HashMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use bollard::Docker;
use console::style;
use log::{debug, info};

use brane_cfg::node::{CentralPaths, CentralPorts, CommonPaths, NodeConfig, NodeKind, NodeKindConfig, WorkerPaths, WorkerPorts};
use brane_tsk::docker::{ensure_image, get_digest};
use specifications::container::Image;
use specifications::version::Version;

pub use crate::errors::LifetimeError as Error;
use crate::spec::{DockerClientVersion, StartSubcommand};


/***** HELPER FUNCTIONS *****/
/// Makes the given path canonical, casting the error for convenience.
/// 
/// # Arguments
/// - `path`: The path to make canonical.
/// 
/// # Returns
/// The same path but canonical.
/// 
/// # Errors
/// This function errors if we failed to make the path canonical (i.e., something did not exist).
#[inline]
fn canonicalize(path: impl AsRef<Path>) -> Result<PathBuf, Error> {
    let path: &Path = path.as_ref();
    match path.canonicalize() {
        Ok(path) => Ok(path),
        Err(err) => Err(Error::CanonicalizeError{ path: path.into(), err }),
    }
}

/// Resolves the given path to replace '$NODE' with the actual node type.
/// 
/// # Arguments
/// - `path`: The path to resolve.
/// - `node`: Some node-dependent identifier already handled.
/// 
/// # Returns
/// A new PathBuf that is the same but now without $NODE.
#[inline]
fn resolve_node(path: impl AsRef<Path>, node: impl AsRef<str>) -> PathBuf {
    PathBuf::from(path.as_ref().to_string_lossy().replace("$NODE", node.as_ref()))
}

/// Loads the given images.
/// 
/// # Arguments
/// - `docker`: The already connected Docker daemon.
/// - `images`: The map of image name -> image paths to load.
/// - `version`: The Brane version of the images to pull.
/// 
/// # Returns
/// Nothing, but does load them in the local docker daemon if everything goes alright.
/// 
/// # Errors
/// This function errors if the given images could not be loaded.
async fn load_images(docker: &Docker, images: HashMap<impl AsRef<str>, String>, version: &Version) -> Result<(), Error> {
    // Iterate over the images
    for (name, from) in images {
        let name: &str = name.as_ref();

        // Determine whether to pull as file or as a repo thing
        let image_path: PathBuf = PathBuf::from(&from);
        let (image, image_path): (Image, Option<PathBuf>) = if image_path.exists() {
            println!("Loading image {} from file {}...", style(name).green().bold(), style(image_path.display().to_string()).bold());

            // Load the digest, too
            let digest: String = match get_digest(&image_path).await {
                Ok(digest) => digest,
                Err(err)   => { return Err(Error::ImageDigestError{ path: image_path, err }); },
            };

            // Return it
            (Image::new(name, Some(version), Some(digest)), Some(image_path))
        } else {
            println!("Loading image {} from repository {}...", style(name).green().bold(), style(&from).bold());
            (Image::new(&from, None::<&str>, None::<&str>), None)
        };

        // Simply rely on ensure_image
        if let Err(err) = ensure_image(docker, image, image_path).await { return Err(Error::ImageLoadError{ from, err }); }
    }

    // Done
    Ok(())
}

/// Constructs the environment variables for Docker compose.
/// 
/// # Arguments
/// - `version`: The Brane version to launch.
/// - `node_config_path`: The path of the NodeConfig file to mount.
/// - `node_config`: The NodeConfig to set ports and attach volumes for.
/// 
/// # Returns
/// A HashMap of environment variables to use for running Docker compose.
/// 
/// # Errors
/// This function errors if we fail to canonicalize any of the paths in `node_config`.
fn construct_envs(version: &Version, node_config_path: &Path, node_config: &NodeConfig) -> Result<HashMap<&'static str, OsString>, Error> {
    // Set the global ones first
    let mut res: HashMap<&str, OsString> = HashMap::from([
        ("BRANE_VERSION", OsString::from(version.to_string())),
        ("NODE_CONFIG_PATH", node_config_path.as_os_str().into()),
    ]);

    // Match on the node kind
    match &node_config.node {
        NodeKindConfig::Central(central) => {
            // Now we do a little ugly something, but we unpack the paths and ports here so that we get compile errors if we add more later on
            let CommonPaths{ certs, packages } = &node_config.paths;
            let CentralPaths{ infra, secrets } = &central.paths;
            let CentralPorts{ api, drv }       = &central.ports;

            // Add the environment variables, which are basically just central-specific paths and ports to mount in the compose file
            res.extend([
                // Paths
                ("INFRA", canonicalize(infra)?.as_os_str().into()),
                ("SECRETS", canonicalize(secrets)?.as_os_str().into()),
                ("CERTS", canonicalize(certs)?.as_os_str().into()),
                ("PACKAGES", canonicalize(packages)?.as_os_str().into()),
    
                // Ports
                ("API_PORT", OsString::from(format!("{}", api.port()))),
                ("DRV_PORT", OsString::from(format!("{}", drv.port()))),
            ]);
        },

        NodeKindConfig::Worker(worker) => {
            // Now we do a little ugly something, but we unpack the paths here so that we get compile errors if we add more later on
            let CommonPaths{ certs, packages }                               = &node_config.paths;
            let WorkerPaths{ creds, data, results, temp_data, temp_results } = &worker.paths;
            let WorkerPorts{ reg, job }                                      = &worker.ports;

            // Add the environment variables, which are basically just central-specific paths to mount in the compose file
            res.extend([
                // Paths
                ("CREDS", canonicalize(creds)?.as_os_str().into()),
                ("CERTS", canonicalize(certs)?.as_os_str().into()),
                ("PACKAGES", canonicalize(packages)?.as_os_str().into()),
                ("DATA", canonicalize(data)?.as_os_str().into()),
                ("RESULTS", canonicalize(results)?.as_os_str().into()),
                ("TEMP_DATA", canonicalize(temp_data)?.as_os_str().into()),
                ("TEMP_RESULTS", canonicalize(temp_results)?.as_os_str().into()),

                // Ports
                ("REG_PORT", OsString::from(format!("{}", reg.port()))),
                ("JOB_PORT", OsString::from(format!("{}", job.port()))),
            ]);
        },
    }

    // Done
    Ok(res)
}

/// Runs Docker compose on the given Docker file.
/// 
/// # Arguments
/// - `file`: The DockerFile to run.
/// - `project`: The project name to launch the containers for.
/// - `envs`: The map of environment variables to set.
/// 
/// # Returns
/// Nothing upon success, although obviously the Docker containers do get launched if so.
/// 
/// # Errors
/// This function fails if we failed to launch the command, or the command itself failed.
fn run_compose(file: impl AsRef<Path>, project: impl AsRef<str>, envs: HashMap<&'static str, OsString>) -> Result<(), Error> {
    let file    : &Path = file.as_ref();
    let project : &str  = project.as_ref();

    // Start creating the command
    let mut cmd: Command = Command::new("docker-compose");
    cmd.stdin(Stdio::inherit());
    cmd.stdout(Stdio::inherit());
    cmd.stderr(Stdio::inherit());
    cmd.args([ "-p", project, "-f" ]);
    cmd.arg(file.as_os_str());
    cmd.args([ "up", "-d" ]);
    cmd.envs(envs);

    // Run it
    println!("Running docker-compose on {}...", style(file.display()).bold().green());
    debug!("Command: {:?}", cmd);
    let output: Output = match cmd.output() {
        Ok(output) => output,
        Err(err)   => { return Err(Error::JobLaunchError { command: cmd, err }); },
    };
    if !output.status.success() { return Err(Error::JobFailure { command: cmd, status: output.status }); }

    // Done
    Ok(())
}





/***** LIBRARY *****/
/// Starts the local node by running the given docker-compose file.
/// 
/// # Arguments
/// - `file`: The `docker-compose.yml` file to launch.
/// - `docker_socket`: The Docker socket path to connect through.
/// - `docker_version`: The Docker client API version to use.
/// - `version`: The Brane version to start.
/// - `node_config_path`: The path to the node config file to potentially override.
/// - `command`: The `StartSubcommand` that carries additional information, including which of the node types to launch.
/// 
/// # Returns
/// Nothing, but does change the local Docker daemon to load and then run the given files.
/// 
/// # Errors
/// This function errors if we failed to run the `docker-compose` command or if we failed to assert that the given command matches the node kind of the `node.yml` file on disk.
pub async fn start(file: impl Into<PathBuf>, docker_socket: PathBuf, docker_version: DockerClientVersion, version: Version, node_config_path: impl Into<PathBuf>, command: StartSubcommand) -> Result<(), Error> {
    let file             : PathBuf = file.into();
    let node_config_path : PathBuf = node_config_path.into();
    info!("Starting node from Docker compose file '{}', defined in '{}'", file.display(), node_config_path.display());

    // Start by loading the node config file
    debug!("Loading node config file '{}'...", node_config_path.display());
    let node_config: NodeConfig = match NodeConfig::from_path(&node_config_path) {
        Ok(config) => config,
        Err(err)   => { return Err(Error::NodeConfigLoadError{ err }); },
    };

    // Match on the command
    match command {
        StartSubcommand::Central{ aux_kafka } => {
            // Assert we are building the correct one
            if node_config.node.kind() != NodeKind::Central { return Err(Error::UnmatchedNodeKind{ got: NodeKind::Central, expected: node_config.node.kind() }); }

            // Connect to the Docker client
            let docker: Docker = match Docker::connect_with_unix(&docker_socket.to_string_lossy(), 120, &docker_version.0) {
                Ok(docker) => docker,
                Err(err)   => { return Err(Error::DockerConnectError{ socket: docker_socket, version: docker_version.0, err }); },
            };

            // Map the images & load them
            let images: HashMap<&'static str, String> = HashMap::from([
                ("aux-kafka", aux_kafka),
            ]);
            load_images(&docker, images, &version).await?;

            // Construct the environment variables
            let envs: HashMap<&str, OsString> = construct_envs(&version, &node_config_path, &node_config)?;

            // Launch the docker-compose command
            run_compose(resolve_node(file, "central"), "brane-central", envs)?;
        },

        StartSubcommand::Worker {} => {
            // Assert we are building the correct one
            if node_config.node.kind() != NodeKind::Worker  { return Err(Error::UnmatchedNodeKind{ got: NodeKind::Worker, expected: node_config.node.kind() }); }

            // Connect to the Docker client
            let docker: Docker = match Docker::connect_with_unix(&docker_socket.to_string_lossy(), 120, &docker_version.0) {
                Ok(docker) => docker,
                Err(err)   => { return Err(Error::DockerConnectError{ socket: docker_socket, version: docker_version.0, err }); },
            };

            // Map the images & load them
            let images: HashMap<&'static str, String> = HashMap::from([]);
            load_images(&docker, images, &version).await?;

            // Construct the environment variables
            let envs: HashMap<&str, OsString> = construct_envs(&version, &node_config_path, &node_config)?;

            // Launch the docker-compose command
            run_compose(resolve_node(file, "worker"), format!("brane-worker-{}", node_config.node.worker().location_id), envs)?;
        },
    }

    // Done
    println!("\nSuccessfully launched node of type {}", style(node_config.node.kind()).bold().green());
    Ok(())
}
