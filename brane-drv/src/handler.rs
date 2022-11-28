//  HANDLER.rs
//    by Lut99
// 
//  Created:
//    12 Sep 2022, 16:18:11
//  Last edited:
//    28 Nov 2022, 16:27:45
//  Auto updated?
//    Yes
// 
//  Description:
//!   Implements the command handler from the client.
// 

use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;

use dashmap::DashMap;
use log::{debug, error};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};

use brane_ast::Workflow;
use brane_cfg::node::NodeConfig;
use brane_exe::FullValue;
use brane_tsk::spec::{AppId, Planner};
use brane_tsk::grpc;

use crate::errors::RemoteVmError;
use crate::planner::InstancePlanner;
use crate::vm::InstanceVm;


/***** HELPER MACROS *****/
/// Sends an error back to the client, also logging it here. Is like `err!` but returning the stream.
macro_rules! fatal_err {
    ($tx:ident, Status::$status:ident, $err:expr) => {
        {
            // Always log to stderr
            log::error!("{}", $err);
            // Attempt to log on tx
            let serr: String = $err.to_string();
            if let Err(err) = $tx.send(Err(Status::$status(serr))).await { log::error!("Failed to notify client of error: {}", err); }
            // Return
            return;
        }
    };
    ($tx:ident, $status:expr) => {
        {
            // Always log to stderr
            log::error!("Aborting incoming request: {}", $status);
            // Attempt to log on tx
            if let Err(err) = $tx.send(Err($status)).await { log::error!("Failed to notify client of error: {}", err); }
            // Return
            return;
        }
    };

    ($tx:ident, $rx:ident, Status::$status:ident, $err:expr) => {
        {
            // Always log to stderr
            log::error!("{}", $err);
            // Attempt to log on tx
            if let Err(err) = $tx.send(Err(Status::$status($err.to_string()))).await { log::error!("Failed to notify client of error: {}", err); }
            // Return
            return Ok(Response::new(ReceiverStream::new($rx)));
        }
    };
    ($tx:ident, $rx:ident, $status:expr) => {
        {
            // Always log to stderr
            log::error!("Aborting incoming request: {}", $status);
            // Attempt to log on tx
            if let Err(err) = $tx.send(Err($status)).await { log::error!("Failed to notify client of error: {}", err); }
            // Return
            return Ok(Response::new(ReceiverStream::new($rx)));
        }
    };
}





/***** LIBRARY *****/
/// The DriverHandler handles incoming gRPC requests. This is effectively what 'drives' the driver.
#[derive(Clone)]
pub struct DriverHandler {
    /// The path to the `node.yml` file that describes this node's environment. For the handler, this is the path to the `infra.yml` file (and an optional `secrets.yml`) and the topic to send commands to the planner on.
    node_config_path : PathBuf,
    /// The planner we use to plan stuff.
    planner          : Arc<InstancePlanner>,

    /// Current sessions and active VMs. Note that this only concerns states if connected via a REPL-session; any in-statement state (i.e., calling nodes) is handled by virtue of the VM being implemented as `async`.
    sessions : Arc<DashMap<AppId, InstanceVm>>,
}

impl DriverHandler {
    /// Constructor for the DriverHandler.
    /// 
    /// # Arguments
    /// - `node_config_path`: The path to the `node.yml` file that describes this node's environment. For the handler, this is the path to the `infra.yml` file (and an optional `secrets.yml`) and the topic to send commands to the planner on.
    /// - `planner`: The InstancePlanner that handles our side of planning.
    /// 
    /// # Returns
    /// A new DriverHandler instance.
    #[inline]
    pub fn new(node_config_path: impl Into<PathBuf>, planner: Arc<InstancePlanner>) -> Self {
        Self {
            node_config_path : node_config_path.into(),
            planner,

            sessions : Arc::new(DashMap::new()),
        }
    }
}

#[tonic::async_trait]
impl grpc::DriverService for DriverHandler {
    type ExecuteStream = ReceiverStream<Result<grpc::ExecuteReply, Status>>;

    /// Creates a new BraneScript session.
    /// 
    /// # Arguments
    /// - `request`: The request to create a response to.
    /// 
    /// # Returns
    /// The response to the request, which only contains a new AppId.
    /// 
    /// # Errors
    /// This function doesn't typically error.
    async fn create_session(&self, _request: Request<grpc::CreateSessionRequest>) -> Result<Response<grpc::CreateSessionReply>, Status> {
        // Create a new VM for this session
        let app_id: AppId = AppId::generate();
        self.sessions.insert(app_id.clone(), InstanceVm::new(&self.node_config_path, app_id.clone(), self.planner.clone()));

        // Now return the ID to the user for future reference
        debug!("Created new session '{}'", app_id);
        let reply = grpc::CreateSessionReply { uuid: app_id.into() };
        Ok(Response::new(reply))
    }



    /// Executes a new job in an existing BraneScript session.
    /// 
    /// # Arguments
    /// - `request`: The request with the new (already compiled) snippet to execute.
    /// 
    /// # Returns
    /// The response to the request, which contains the result of this workflow (if any).
    /// 
    /// # Errors
    /// This function may error for any reason a job might fail.
    async fn execute(&self, request: Request<grpc::ExecuteRequest>) -> Result<Response<Self::ExecuteStream>, Status> {
        let request = request.into_inner();
        debug!("Receiving execute request for session '{}'", request.uuid);

        // Prepare gRPC stream between client and (this) driver.
        let (tx, rx) = mpsc::channel::<Result<grpc::ExecuteReply, Status>>(10);

        // Parse the given ID
        let app_id: AppId = match AppId::from_str(&request.uuid) {
            Ok(app_id) => app_id,
            Err(err)   => { fatal_err!(tx, rx, Status::invalid_argument, err); },
        };

        // Load the config, making sure it's a central config
        let node_config: NodeConfig = match NodeConfig::from_path(&self.node_config_path) {
            Ok(config) => config,
            Err(err)   => {
                error!("Failed to load the NodeConfig: {}", err);
                fatal_err!(tx, rx, Status::internal, "An internal error has occurred.");
            },
        };

        // Fetch the VM
        let sessions: Arc<DashMap<AppId, InstanceVm>> = self.sessions.clone();
        let vm: InstanceVm = match sessions.get(&app_id) {
            Some(vm) => vm.clone(),
            None     => { fatal_err!(tx, rx, Status::internal(format!("No session with ID '{}' found", app_id))); }
        };

        // We're gonna run the rest asynchronous, to allow the client to earlier receive callbacks
        let planner: Arc<InstancePlanner> = self.planner.clone();
        tokio::spawn(async move {
            debug!("Executing workflow for session '{}'", app_id);
    
            // We assume that the input is an already compiled workflow; so no need to fire up any parsers/compilers

            // We only have to use JSON magic
            debug!("Parsing workflow of {} characters", request.input.len());
            let workflow: Workflow = match serde_json::from_str(&request.input) {
                Ok(workflow) => workflow,
                Err(err)     => {
                    debug!("Workflow:\n{}\n{}\n{}\n\n", (0..80).map(|_| '-').collect::<String>(), request.input, (0..80).map(|_| '-').collect::<String>());
                    fatal_err!(tx, Status::invalid_argument, err);
                },
            };

            // Spend some time resolving the workflow with the planner
            debug!("Planning workflow on Kafka topic '{}'", node_config.node.central().topics.planner_command);
            let plan: Workflow = match planner.plan(workflow).await {
                Ok(plan) => plan,
                Err(err) => { fatal_err!(tx, Status::internal, err); },
            };

            // We now have a runnable plan ( ͡° ͜ʖ ͡°), so run it
            debug!("Executing workflow of {} edges", plan.graph.len());
            let (vm, res): (InstanceVm, Result<FullValue, RemoteVmError>) = vm.exec(tx.clone(), plan).await;

            // Insert the VM again
            debug!("Saving state session state");
            sessions.insert(app_id, vm);

            // Switch on the actual result and send that back to the user
            match res {
                Ok(res)  => {
                    debug!("Completed execution.");

                    // Serialize the value
                    let sres: String = match serde_json::to_string(&res) {
                        Ok(sres) => sres,
                        Err(err) => { fatal_err!(tx, Status::internal, err); }  
                    };

                    // Create the reply text
                    let msg = String::from("Driver completed execution.");
                    let reply = grpc::ExecuteReply {
                        close  : true,
                        debug  : Some(msg.clone()),
                        stderr : None,
                        stdout : None,
                        value  : Some(sres),
                    };

                    // Send it
                    if let Err(err) = tx.send(Ok(reply)).await {
                        error!("Failed to send workflow result back to client: {}", err);
                    }
                },
                Err(err) => {
                    fatal_err!(tx, Status::internal, err);
                },
            };
        });

        // Return the receiver stream so the client can find us
        Ok(Response::new(ReceiverStream::new(rx)))
    }
}
