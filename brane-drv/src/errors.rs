//  ERRORS.rs
//    by Lut99
// 
//  Created:
//    01 Feb 2022, 16:13:53
//  Last edited:
//    03 Nov 2022, 14:42:45
//  Auto updated?
//    Yes
// 
//  Description:
//!   Contains errors used within the brane-drv package only.
// 

use std::error::Error;
use std::fmt::{Display, Formatter, Result as FResult};

use rdkafka::error::KafkaError;
use rdkafka::error::RDKafkaErrorCode;
use tonic::Status;

use brane_tsk::spec::JobStatus;
use brane_tsk::grpc::{ExecuteReply, TaskStatus};
use specifications::version::Version;


/***** ERRORS *****/
/// Errors that occur during the main phase of the brane-drv package
#[derive(Debug)]
pub enum DriverError {
    /// Could not create a Kafka client
    KafkaClientError{ servers: String, err: KafkaError },
    /// Could not get the Kafka client to try to add more topics
    KafkaTopicsError{ topics: String, err: KafkaError },
    /// Could not add the given topic (with a duplicate error already filtered out)
    KafkaTopicError{ topic: String, err: RDKafkaErrorCode },
    /// Could not create a Kafka consumer
    KafkaConsumerError{ servers: String, id: String, err: KafkaError },

    /// Could not get the Kafka commit offsets
    KafkaGetOffsetError{ topic: String, err: KafkaError },
    /// Could not update the Kafka commit offsets
    KafkaSetOffsetError{ topic: String, err: KafkaError },
    /// Could not commit the update to the Kafka commit offsets
    KafkaSetOffsetsError{ topic: String, err: KafkaError },

    /// Failed to decode an incomming PlanningUpdate message.
    PlanningUpdateDecodeError{ topic: String, err: prost::DecodeError },
    /// A plan was missing in a Success message
    MissingPlanError{ topic: String, correlation_id: String },
    /// An unknown planning status kind was given.
    UnknownPlanningStatusKind{ topic: String, correlation_id: String, raw: i32 },
    /// Error for when we failed to monitor events
    EventMonitorError{ err: KafkaError },
}

impl DriverError {
    /// Serializes a given list of vectors into a string.
    /// 
    /// **Generic types**
    ///  * `T`: The type of the vector. Must be convertible to string via the Display trait.
    /// 
    /// **Arguments**
    ///  * `v`: The Vec to serialize.
    /// 
    /// **Returns**  
    /// A string describing the vector. Nothing too fancy, just a list separated by commas.
    pub fn serialize_vec<T>(v: &[T]) -> String
    where
        T: Display
    {
        let mut res: String = String::new();
        for e in v {
            if res.is_empty() { res += ", "; }
            res += &format!("'{}'", e);
        }
        res
    }
}

impl Display for DriverError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FResult {
        use DriverError::*;
        match self {
            KafkaClientError{ servers, err }       => write!(f, "Could not create Kafka client with bootstrap servers '{}': {}", servers, err),
            KafkaTopicsError{ topics, err }        => write!(f, "Could not create new Kafka topics '{}': {}", topics, err),
            KafkaTopicError{ topic, err }          => write!(f, "Could not create Kafka topic '{}': {}", topic, err),
            KafkaConsumerError{ servers, id, err } => write!(f, "Could not create Kafka consumer for ID '{}' with bootstrap servers '{}': {}", id, servers, err),

            KafkaGetOffsetError{ topic, err }  => write!(f, "Could not get offsets for topic '{}': {}", topic, err),
            KafkaSetOffsetError{ topic, err }  => write!(f, "Could not set offsets for topic '{}': {}", topic, err),
            KafkaSetOffsetsError{ topic, err } => write!(f, "Could not commit offsets for topic '{}': {}", topic, err),

            PlanningUpdateDecodeError{ topic, err }                 => write!(f, "Failed to decode incoming message on topic '{}' as a PlanningUpdate message: {}", topic, err),
            MissingPlanError{ topic, correlation_id }               => write!(f, "Received a planning success notification for workflow '{}' on topic '{}', but no plan was provided", correlation_id, topic),
            UnknownPlanningStatusKind{ topic, correlation_id, raw } => write!(f, "Received unknown planning update status kind '{}' for workflow '{}', on topic '{}'", raw, correlation_id, topic),
            EventMonitorError{ err }                                => write!(f, "Failed to monitor Kafka events: {}", err),
        }
    }
}

impl Error for DriverError {}



/// Collects errors that relate to the RemoteVm.
#[derive(Debug)]
pub enum RemoteVmError {
    /// No locations were possible to be run from the start.
    NoLocationError,

    /// Failed to get the remote package index.
    PackageIndexError{ address: String, err: brane_tsk::errors::ApiError },
    /// The given package was unknown.
    UnknownPackage{ name: String, version: Version },

    // /// Failed to encode a Command.
    // CommandEncodeError{ err: prost::EncodeError },

    // /// Failed to send a Kafka message.
    // KafkaSendError{ name: String, topic: String, err: rdkafka::error::KafkaError },
    // /// The external call failed.
    // ExternalCallFailed{ name: String, package: String, version: Version, code: i32, stdout: String, stderr: String },
    // /// The external call had a non-package reason for failing.
    // ExternalCallError{ name: String, package: String, version: Version, err: String },

    /// Failed to serialize the given arguments.
    ArgsJsonEncodeError{ err: serde_json::Error },
    /// Failed to establish a gRPC connection to the remote Job node.
    GrpcConnectError{ endpoint: String, err: tonic::transport::Error },
    /// Failed to send the execute request the remote Job node.
    GrpcRequestError{ endpoint: String, err: tonic::Status },
    /// The Job node sent us a JobStatus that we should've got from branelet instead.
    IllegalJobStatus{ status: JobStatus },
    // /// Failed to create a task.
    // TaskCreateError{ status: ExecuteStatus, err: String },
    // /// Failed to execute a task.
    // TaskExecuteError{ err: InstanceError },
    /// Failed to create a task.
    TaskExecuteError{ status: TaskStatus, err: String },

    /// Failed to write the stdout to the client
    ExternalStdoutWriteError{ err: tokio::sync::mpsc::error::SendError<Result<ExecuteReply, Status>> },

    /// It was a global-occuring error
    VmError{ err: brane_exe::Error },
}

impl RemoteVmError {
    /// Converts this LocalVmError into a VmError using magic (and the given edge index).
    /// 
    /// # Arguments
    /// - `edge`: The current edge index.
    /// 
    /// # Returns
    /// A new `VmError::Custom` of this LocalVmError.
    #[inline]
    pub fn to_vm(self, edge: usize) -> brane_exe::Error {
        brane_exe::Error::Custom { edge, err: Box::new(self) }
    }
}

impl Display for RemoteVmError {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> FResult {
        use RemoteVmError::*;
        match self {
            NoLocationError => write!(f, "No locations are allowed; cannot run job"),

            PackageIndexError{ address, err } => write!(f, "Failed to get instance package index from '{}': {}", address, err),
            UnknownPackage{ name, version }   => write!(f, "Unknown package '{}' (or it has no version {})", name, version),

            // CommandEncodeError{ err }  => write!(f, "Failed to encode command to raw bytes: {}", err),

            // KafkaSendError{ name, topic, err }                                 => write!(f, "Failed to send launch command on Kafka for task '{}' on topic '{}': {}", name, topic, err),
            // ExternalCallFailed{ name, package, version, code, stdout, stderr } => write!(f, "Task '{}' (part of package '{}', version {}) failed with exit code {}\n\nstdout:\n{}\n{}\n{}\n\nstderr:\n{}\n{}\n{}\n\n", name, package, version, code, (0..80).map(|_| '-').collect::<String>(), stdout, (0..80).map(|_| '-').collect::<String>(), (0..80).map(|_| '-').collect::<String>(), stderr, (0..80).map(|_| '-').collect::<String>()),
            // ExternalCallError{ name, package, version, err }                   => write!(f, "Task '{}' (part of package '{}', version {}) failed to be executed: {}", name, package, version, err),

            ArgsJsonEncodeError{ err }        => write!(f, "Failed to serialize task arguments: {}", err),
            GrpcConnectError{ endpoint, err } => write!(f, "Failed to establish gRPC connection to '{}': {}", endpoint, err),
            GrpcRequestError{ endpoint, err } => write!(f, "Failed to send job execute request to '{}': {}", endpoint, err),
            IllegalJobStatus{ status }        => write!(f, "Received job status {:?} from `brane-job`, whereas it should have come in as a callback", TaskStatus::from(status)),
            // TaskCreateError{ status, err }    => write!(f, "Job creation failed with status {:?}: {}", status, err),
            // TaskExecuteError{ err }           => write!(f, "Job failed: {}", err),
            TaskExecuteError{ status, err }   => write!(f, "Job creation failed with status {:?}: {}", status, err),

            ExternalStdoutWriteError{ err } => write!(f, "Failed to write to remote stdout: {}", err),

            VmError{ err } => write!(f, "{}", err),
        }
    }
}

impl Error for RemoteVmError {}



/// An error has occurred while interacting with the backend instance.
#[derive(Debug)]
pub enum InstanceError {
    /// The workflow was not picked up by a planner in time.
    PlanningTimeout{ correlation_id: String, timeout: u128 },
    /// We failed to parse a planned workflow.
    PlanParseError{ correlation_id: String, raw: String, err: serde_json::Error },
    /// The planner told us it has failed because it could not come up with possible plans (it optionally gives a reason for that).
    PlanningFailed{ correlation_id: String, failed: Option<String> },
    /// The planner has encountered a fatal error.
    PlanningError{ correlation_id: String, err: String },
    /// We got an unknown plan ID.
    UnknownPlanResultKindError{ correlation_id: String, raw: i32 },

    /// The job has failed.
    JobFailed{ correlation_id: String, code: i32, stdout: String, stderr: String },
    /// The job has stopped unexpectedly.
    JobStopped{ correlation_id: String },
    /// We could not decode the job's output.
    JobDecodeFailed{ correlation_id: String, err: String },
    /// The job did not complete (i.e., process completed successfully).
    JobCompleteFailed{ correlation_id: String, err: String },
    /// The job did not start (i.e., branelet process spawn).
    JobStartFailed{ correlation_id: String, err: String },
    /// The job did not initialize (i.e., branelet startup).
    JobInitializeFailed{ correlation_id: String, err: String },
    /// Failed to create a new job (i.e., container creation failed).
    JobCreateFailed{ correlation_id: String, err: String },

    /// The job did not come with a result in time.
    JobResultTimeout{ correlation_id: String, timeout: u128 },
    /// The job did not send a heartbeat in time.
    JobHeartbeatTimeout{ correlation_id: String, timeout: u128 },
    /// The job was not started in time.
    JobStartedTimeout{ correlation_id: String, timeout: u128 },
    /// The job was not initialized in time.
    JobInitializedTimeout{ correlation_id: String, timeout: u128 },
    /// The job was not ready in time.
    JobReadyTimeout{ correlation_id: String, timeout: u128 },
    /// The job did not created in time.
    JobCreatedTimeout{ correlation_id: String, timeout: u128 },

    /// Failed to deserialize the finished result.
    FinishedDeserializeError{ correlation_id: String, raw: String, err: serde_json::Error },
    /// The job has failed but we also couldn't parse its output.
    FailedDeserializeError{ correlation_id: String, raw: String, err: serde_json::Error },
}

impl Display for InstanceError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FResult {
        use InstanceError::*;
        match self {
            PlanningTimeout{ correlation_id, timeout }        => write!(f, "Planning of workflow '{}' timed out (planner did not pick the task up within {} seconds)", correlation_id, timeout / 1000),
            PlanParseError{ correlation_id, raw, err }        => write!(f, "Failed to parse \"\"\"{}\"\"\" as a plan as a result of planning workflow '{}': {}", raw, correlation_id, err),
            PlanningFailed{ correlation_id, failed }          => write!(f, "Unable to find a possible plan for workflow '{}'{}", correlation_id, if let Some(failed) = failed { format!(": {}", failed) } else { String::new() }),
            PlanningError{ correlation_id, err }              => write!(f, "An error occurred trying to plan workflow '{}': {}", correlation_id, err),
            UnknownPlanResultKindError{ correlation_id, raw } => write!(f, "Unknown plan result identifier '{}' received as result of planning workflow '{}'", raw, correlation_id),

            JobFailed{ correlation_id, code, stdout, stderr } => write!(f, "Job '{}' failed with exit code {}\n\nstdout:\n{}\n{}\n{}\n\nstderr:\n{}\n{}\n{}\n\n", correlation_id, code, (0..80).map(|_| '-').collect::<String>(), stdout, (0..80).map(|_| '-').collect::<String>(), (0..80).map(|_| '-').collect::<String>(), stderr, (0..80).map(|_| '-').collect::<String>()),
            JobStopped{ correlation_id }                      => write!(f, "Job '{}' was interrupted", correlation_id),
            JobDecodeFailed{ correlation_id, err }            => write!(f, "Failed to decode output of job '{}': {}", correlation_id, err),
            JobCompleteFailed{ correlation_id, err }          => write!(f, "Failed to complete job '{}': {}", correlation_id, err),
            JobStartFailed{ correlation_id, err }             => write!(f, "Failed to start job '{}': {}", correlation_id, err),
            JobInitializeFailed{ correlation_id, err }        => write!(f, "Failed to initialize job '{}': {}", correlation_id, err),
            JobCreateFailed{ correlation_id, err }            => write!(f, "Failed to create job '{}': {}", correlation_id, err),

            JobResultTimeout{ correlation_id, timeout }      => write!(f, "Job with ID '{}' failed to send a result within {} seconds (timeout)", correlation_id, timeout / 1000),
            JobHeartbeatTimeout{ correlation_id, timeout }   => write!(f, "Job with ID '{}' failed to send a heartbeat within {} seconds (timeout)", correlation_id, timeout / 1000),
            JobStartedTimeout{ correlation_id, timeout }     => write!(f, "Job with ID '{}' failed to be started within {} seconds (timeout)", correlation_id, timeout / 1000),
            JobInitializedTimeout{ correlation_id, timeout } => write!(f, "Job with ID '{}' failed to be initialized within {} seconds (timeout)", correlation_id, timeout / 1000),
            JobReadyTimeout{ correlation_id, timeout }       => write!(f, "Job with ID '{}' failed to be ready within {} seconds (timeout)", correlation_id, timeout / 1000),
            JobCreatedTimeout{ correlation_id, timeout }     => write!(f, "Job with ID '{}' failed to be created within {} seconds (timeout)", correlation_id, timeout / 1000),

            FinishedDeserializeError{ correlation_id, raw, err } => write!(f, "Failed to deserialize \"\"\"{}\"\"\" (output from job '{}') as a JSON FullValue: {}", raw, correlation_id, err),
            FailedDeserializeError{ correlation_id, raw, err }   => write!(f, "Failed to deserialize reason for failure \"\"\"{}\"\"\" of job '{}': {} (it failed, by the way)", raw, correlation_id, err),
        }
    }
}

impl Error for InstanceError {}



/// Errors that occur during planning.
#[derive(Debug)]
pub enum PlanError {
    /// Failed to serialize a workflow.
    WorkflowSerializeError{ err: serde_json::Error },
    /// Failed to encode the payload.
    PlanEncodeError{ correlation_id: String, err: prost::EncodeError },
    /// Failed to send the Kafka command.
    KafkaSendError{ correlation_id: String, topic: String, err: rdkafka::error::KafkaError },
    /// Failed to (wait for) plan(ning).
    PlanningError{ correlation_id: String, err: InstanceError },
}

impl Display for PlanError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FResult {
        use PlanError::*;
        match self {
            WorkflowSerializeError{ err }                => write!(f, "Failed to serialize given Workflow: {}", err),
            PlanEncodeError{ correlation_id, err }       => write!(f, "Failed to encode planning command for workflow '{}': {}", correlation_id, err),
            KafkaSendError{ correlation_id, topic, err } => write!(f, "Failed to send workflow '{}' for planning on Kafka topic '{}': {}", correlation_id, topic, err),
            PlanningError{ correlation_id, err }         => write!(f, "Failed to plan workflow '{}': {}", correlation_id, err),
        }
    }
}

impl Error for PlanError {}
