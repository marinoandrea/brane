//  MAIN.rs
//    by Lut99
// 
//  Created:
//    18 Oct 2022, 13:47:17
//  Last edited:
//    18 Nov 2022, 15:53:46
//  Auto updated?
//    Yes
// 
//  Description:
//!   Entrypoint to the `brane-job` service.
// 

use std::net::{SocketAddr, ToSocketAddrs};
use std::path::PathBuf;

use clap::{ArgAction::SetTrue, Parser};
use dotenvy::dotenv;
use log::LevelFilter;
use log::{debug, error, info};
use tonic::transport::Server;

use brane_tsk::grpc::JobServiceServer;
use brane_tsk::instance::worker::{EnvironmentInfo, WorkerServer};


#[derive(Parser)]
#[clap(version = env!("CARGO_PKG_VERSION"))]
struct Opts {
    /// Print debug info
    #[clap(long, action=SetTrue, env = "DEBUG")]
    debug           : bool,
    /// Whether to keep containers after execution or not.
    #[clap(long, action=SetTrue, env = "KEEP_CONTAINERS")]
    keep_containers : bool,

    /// The identifier of this location.
    #[clap(short, long, env = "LOCATION_ID")]
    location_id : String,
    /// Service address to service on
    #[clap(short, long, default_value = "127.0.0.1:50052", env = "ADDRESS")]
    address     : String,

    /// If given, then this is the proxy address through which we proxy transfers.
    #[clap(short='P', long, env="PROXY")]
    proxy   : Option<String>,
    /// Local checker endpoint
    #[clap(long, default_value = "http://127.0.0.1:50053", env = "CHECKER_ADDRESS")]
    checker : String,
    /// Xenon gRPC endpoint
    #[clap(short, long, default_value = "http://127.0.0.1:50054", env = "XENON")]
    xenon   : String,

    /// The path where packages are stored after they are downloaded
    #[clap(short, long, default_value="/packages", env="PACKAGES_PATH")]
    packages_path     : PathBuf,
    /// The path where data is stored (shared with registry for speedz). Needs to be externally available to share with spawned containers.
    #[clap(short, long, default_value="/data", env="DATA_PATH")]
    data_path         : PathBuf,
    /// The path where data is stored (shared with registry for speedz). Needs to be externally available to share with spawned containers.
    #[clap(short, long, default_value="/results", env="RESULTS_PATH")]
    results_path      : PathBuf,
    /// The path where data is stored but only for temporary downloads. Needs to be externally available to share with spawned containers.
    #[clap(long, default_value="/temp_data", env="TEMP_DATA_PATH")]
    temp_data_path    : PathBuf,
    /// The path where results are stored but only for temporary downloads. Needs to be externally available to share with spawned containers.
    #[clap(long, default_value="/temp_results", env="TEMP_RESULTS_PATH")]
    temp_results_path : PathBuf,
    /// Credentials metadata store for connecting to the backend (if any)
    #[clap(short, long, default_value = "/config/creds.yml", env = "CREDS")]
    creds             : PathBuf,
    /// Path to the certificates that we use to access everybody.
    #[clap(long, default_value = "/certs", help = "Defines the path to the certificates we use to access registries. Specifically, we expect it to be a directory, with a nested directory named after each location ID for every location we might want to access. Then, in each nested directory, there should be a file 'client.pem', 'client-key.pem' and 'ca.pem'.", env="CERTS")]
    certs             : PathBuf,
}

/* TIM */
/// **Edited: Working with much more structured error handling.**
#[tokio::main]
async fn main() {
    dotenv().ok();
    let opts = Opts::parse();

    // Configure logger.
    let mut logger = env_logger::builder();
    logger.format_module_path(false);

    if opts.debug {
        logger.filter_level(LevelFilter::Debug).init();
    } else {
        logger.filter_level(LevelFilter::Info).init();
    }
    info!("Initializing brane-job...");
    debug!("Data path: '{}'", opts.data_path.display());
    debug!("Results path: '{}'", opts.results_path.display());

    // Initialize the Xenon thingy
    // debug!("Initializing Xenon...");
    // let xenon_schedulers = Arc::new(DashMap::<String, Arc<RwLock<Scheduler>>>::new());
    // let xenon_endpoint = utilities::ensure_http_schema(&opts.xenon, !opts.debug)?;

    // // Create the temporary data folder
    // let temp_data: TempDir = match TempDir::new() {
    //     Ok(temp_data) => temp_data,
    //     Err(err)      => {
    //         error!("Failed to create temporary data folder: {}", err);
    //         std::process::exit(1);
    //     },
    // };

    // Start the JobHandler
    let server = WorkerServer::new(
        EnvironmentInfo::new(
            opts.location_id,
            opts.creds,
            opts.certs,
            opts.packages_path,
            opts.data_path,
            opts.results_path,
            opts.temp_data_path,
            opts.temp_results_path,
            opts.checker,
            opts.proxy,
            opts.keep_containers,
        ),
    );

    // Parse the socket address(es)
    let addr: SocketAddr = match opts.address.to_socket_addrs() {
        Ok(mut addr) => match addr.next() {
            Some(addr) => addr,
            None       => { error!("Missing socket address in '{}'", opts.address); std::process::exit(1); }
        },
        Err(err) => { error!("Failed to parse '{}' as a socket address: {}", opts.address, err); std::process::exit(1); },
    };

    // Start gRPC server with callback service.
    debug!("Starting gRPC server servering @ '{}'", addr);
    if let Err(err) = Server::builder()
        .add_service(JobServiceServer::new(server))
        .serve(addr)
        .await
    {
        error!("Failed to start gRPC server: {}", err);
        std::process::exit(1);
    }
}
/*******/

/* TIM */
// /// **Edited: now returns JobErrors.**
// /// 
// /// Makes sure the required topics are present and watched in the local Kafka server.
// /// 
// /// **Arguments**
// ///  * `topics`: The list of topics to make sure they exist of.
// ///  * `brokers`: The string list of Kafka servers that act as the brokers.
// /// 
// /// **Returns**  
// /// Nothing on success, or an ExecutorError otherwise.
// async fn ensure_topics(
//     topics: Vec<&str>,
//     brokers: &str,
// ) -> Result<(), JobError> {
//     // Connect with an admin client
//     let admin_client: AdminClient<_> = match ClientConfig::new().set("bootstrap.servers", brokers) .create() {
//         Ok(client)  => client,
//         Err(reason) => { return Err(JobError::KafkaClientError{ servers: brokers.to_string(), err: reason }); }
//     };

//     // Collect the topics to create and then create them
//     let ktopics: Vec<NewTopic> = topics
//         .iter()
//         .map(|t| NewTopic::new(t, 1, TopicReplication::Fixed(1)))
//         .collect();
//     let results = match admin_client.create_topics(ktopics.iter(), &AdminOptions::new()).await {
//         Ok(results) => results,
//         Err(reason) => { return Err(JobError::KafkaTopicsError{ topics: JobError::serialize_vec(&topics), err: reason }); }
//     };

//     // Report on the results. Don't consider 'TopicAlreadyExists' an error.
//     for result in results {
//         match result {
//             Ok(topic) => info!("Kafka topic '{}' created.", topic),
//             Err((topic, error)) => match error {
//                 RDKafkaErrorCode::TopicAlreadyExists => {
//                     info!("Kafka topic '{}' already exists", topic);
//                 }
//                 _ => { return Err(JobError::KafkaTopicError{ topic, err: error }); }
//             },
//         }
//     }

//     Ok(())
// }
/*******/

// /* TIM */
// /// **Edited: Now working with the various errors.**
// /// 
// /// One of the workers in the brane-job service.
// /// 
// /// **Arguments**
// ///  * `debug`: Whether or not to enable debug mode (i.e., more prints and things like not destroying containers)
// ///  * `brokers`: The list of Kafka brokers we're using.
// ///  * `group_id`: The Kafka group ID for the brane-job service.
// ///  * `clb_topic`: The Kafka callback topic for job results.
// ///  * `cmd_topic`: The Kafka command topic for incoming commands.
// ///  * `evt_topic`: The Kafka event topic where we report back to the driver.
// ///  * `infra`: The Infrastructure handle to the infra.yml.
// ///  * `secrets`: The Secrets handle to the infra.yml.
// ///  * `xenon_endpoint`: The Xenon endpoint to connect to and schedule jobs on.
// ///  * `xenon_schedulers`: A list of Xenon schedulers we use to determine where to run what.
// /// 
// /// **Returns**  
// /// Nothing if the worker exited cleanly, or a JobError if it didn't.
// #[allow(clippy::too_many_arguments)]
// async fn start_worker(
//     debug: bool,
//     brokers: String,
//     group_id: String,
//     clb_topic: String,
//     cmd_topic: String,
//     evt_topic: String,
//     infra: Infrastructure,
//     secrets: Secrets,
//     xenon_endpoint: String,
//     xenon_schedulers: Arc<DashMap<String, Arc<RwLock<Scheduler>>>>,
// ) -> Result<(), JobError> {
//     let output_topic = evt_topic.as_ref();

//     debug!("Creating Kafka producer...");
//     let producer: FutureProducer = match ClientConfig::new()
//         .set("bootstrap.servers", &brokers)
//         .set("message.timeout.ms", "5000")
//         .create()
//     {
//         Ok(producer) => producer,
//         Err(reason)  => { return Err(JobError::KafkaProducerError{ servers: brokers, err: reason }); }
//     };

//     debug!("Creating Kafka consumer...");
//     let consumer: StreamConsumer = match ClientConfig::new()
//         .set("group.id", &group_id)
//         .set("bootstrap.servers", &brokers)
//         .set("enable.partition.eof", "false")
//         .set("session.timeout.ms", "6000")
//         .set("enable.auto.commit", "false")
//         .create()
//     {
//         Ok(consumer) => consumer,
//         Err(reason)  => { return Err(JobError::KafkaConsumerError{ servers: brokers, id: group_id, err: reason }); }
//     };

//     // TODO: make use of transactions / exactly-once semantics (EOS)

//     // Restore previous topic/partition offset.
//     let mut tpl = TopicPartitionList::new();
//     tpl.add_partition(&clb_topic, 0);
//     tpl.add_partition(&cmd_topic, 0);

//     let committed_offsets = match consumer.committed_offsets(tpl.clone(), Timeout::Never) {
//         Ok(commited_offsets) => commited_offsets.to_topic_map(),
//         Err(reason)          => { return Err(JobError::KafkaGetOffsetError{ clb: clb_topic, cmd: cmd_topic, err: reason }); }
//     };
//     if let Some(offset) = committed_offsets.get(&(clb_topic.clone(), 0)) {
//         let res = match offset {
//             Offset::Invalid => tpl.set_partition_offset(&clb_topic, 0, Offset::Beginning),
//             offset => tpl.set_partition_offset(&clb_topic, 0, *offset),
//         };
//         if let Err(reason) = res {
//             return Err(JobError::KafkaSetOffsetError{ topic: clb_topic, kind: "callback".to_string(), err: reason });
//         }
//     }
//     if let Some(offset) = committed_offsets.get(&(cmd_topic.clone(), 0)) {
//         let res = match offset {
//             Offset::Invalid => tpl.set_partition_offset(&cmd_topic, 0, Offset::Beginning),
//             offset => tpl.set_partition_offset(&cmd_topic, 0, *offset),
//         };
//         if let Err(reason) = res {
//             return Err(JobError::KafkaSetOffsetError{ topic: cmd_topic, kind: "command".to_string(), err: reason });
//         }
//     }

//     info!("Restoring commited offsets: {:?}", &tpl);
//     if let Err(reason) = consumer.assign(&tpl) {
//         return Err(JobError::KafkaSetOffsetsError{ clb: clb_topic, cmd: cmd_topic, err: reason });
//     }

//     // Create the outer pipeline on the message stream.
//     debug!("Waiting for messages...");
//     let stream_processor = consumer.stream().try_for_each(|borrowed_message| {
//         // Copy the message into owned space
//         consumer.commit_message(&borrowed_message, CommitMode::Sync).unwrap();

//         let owned_message = borrowed_message.detach();
//         let owned_producer = producer.clone();
//         let owned_infra = infra.clone();
//         let owned_secrets = secrets.clone();
//         let owned_xenon_endpoint = xenon_endpoint.clone();
//         let owned_xenon_schedulers = xenon_schedulers.clone();
//         let clb_topic = clb_topic.clone();
//         let cmd_topic = cmd_topic.clone();

//         async move {
//             // Get the message key
//             let msg_key = match owned_message
//                 .key()
//                 .map(String::from_utf8_lossy)
//                 .map(String::from)
//             {
//                 Some(msg_key) => msg_key,
//                 None          => {
//                     warn!("Received message without a key; ignoring message");
//                     return Ok(());
//                 }
//             };

//             // Get the payload
//             let msg_payload = match owned_message.payload() {
//                 Some(msg_payload) => msg_payload,
//                 None              => {
//                     warn!("Received message (key: {}) without a payload; ignoring message", msg_key);
//                     return Ok(());
//                 }
//             };

//             // Depending on the message's topic, handle it differently
//             let topic = owned_message.topic();
//             let events = if topic == clb_topic {
//                 handle_clb_message(msg_key, msg_payload)
//             } else if topic == cmd_topic {
//                 handle_cmd_message(
//                     debug,
//                     msg_key,
//                     msg_payload,
//                     owned_infra,
//                     owned_secrets,
//                     owned_xenon_endpoint,
//                     owned_xenon_schedulers,
//                 )
//                 .await
//             } else {
//                 warn!("Received message (key: {}) with unknown topic '{}'; ignoring message", msg_key, topic);
//                 return Ok(());
//             };

//             // Match the events to return
//             match events {
//                 Ok(events) => {
//                     for (evt_key, event) in events {
//                         // Encode event message into a payload (bytes)
//                         let mut payload = BytesMut::with_capacity(64);
//                         match event.encode(&mut payload) {
//                             Ok(_) => {
//                                 // Send event on output topic
//                                 let message = FutureRecord::to(output_topic).key(&evt_key).payload(payload.to_bytes());
//                                 if let Err(error) = owned_producer.send(message, Timeout::Never).await {
//                                     error!("Failed to send event (key: {}): {:?}", evt_key, error);
//                                 }
//                             },
//                             Err(reason) => { error!("Failed to send event (key: {}): {}", evt_key.clone(), JobError::EventEncodeError{ key: evt_key, err: reason }); }
//                         }
//                     }
//                 }
//                 Err(err) => {
//                     // Log the error but continue listening
//                     error!("{}", &err);
//                 }
//             };

//             Ok(())
//         }
//     });

//     match stream_processor.await {
//         Ok(_)  => Ok(()),
//         Err(_) => panic!("The Stream Processor shouldn't return an error, but it does; this should never happen!"),
//     }
// }
// /*******/

// /* TIM */
// /// **Edited: now returning JobErrors.**
// /// 
// /// Handles a given callback message by calling the appropriate handler.
// /// 
// /// **Arguments**
// ///  * `key`: The key of the message we received.
// ///  * `payload`: The raw, binary payload of the message.
// /// 
// /// **Returns**  
// /// A list of events that should be fired on success, or a JobError if that somehow failed.
// fn handle_clb_message(
//     key: String,
//     payload: &[u8],
// ) -> Result<Vec<(String, Event)>, JobError> {
//     // Decode payload into a callback message.
//     debug!("Decoding clb message...");
//     let callback = match Callback::decode(payload) {
//         Ok(callback) => callback,
//         Err(reason)  => { return Err(JobError::CallbackDecodeError{ key, err: reason }); }
//     };
//     let kind = match CallbackKind::from_i32(callback.kind) {
//         Some(kind) => kind,
//         None       => { return Err(JobError::IllegalCallbackKind{ kind: callback.kind }); }
//     };

//     // Ignore unkown callbacks, as we can't dispatch it.
//     if kind == CallbackKind::Unknown {
//         warn!("Received UNKOWN command (key: {}); ignoring message", key);
//         return Ok(vec![]);
//     }

//     info!("Received {} callback (key: {}).", kind, key);
//     debug!("{:?}", callback);

//     // Call the handler
//     clb_lifecycle::handle(callback)
// }
// /*******/

// /* TIM */
// /// **Edited: now returning JobErrors.**
// /// 
// /// Handles a given command message by calling the appropriate handler.
// /// 
// /// **Arguments**
// ///  * `debug`: Whether or not to enable debug mode (i.e., more prints and things like not destroying containers)
// ///  * `key`: The key of the message we received.
// ///  * `payload`: The raw, binary payload of the message.
// ///  * `infra`: The Infrastructure handle to the infra.yml.
// ///  * `secrets`: The Secrets handle to the infra.yml.
// ///  * `xenon_endpoint`: The Xenon endpoint to connect to and schedule jobs on.
// ///  * `xenon_schedulers`: A list of Xenon schedulers we use to determine where to run what.
// /// 
// /// **Returns**  
// /// A list of events that should be fired on success, or a JobError if that somehow failed.
// async fn handle_cmd_message(
//     debug: bool,
//     key: String,
//     payload: &[u8],
//     infra: Infrastructure,
//     secrets: Secrets,
//     xenon_endpoint: String,
//     xenon_schedulers: Arc<DashMap<String, Arc<RwLock<Scheduler>>>>,
// ) -> Result<Vec<(String, Event)>, JobError> {
//     // Decode payload into a command message.
//     debug!("Decoding cmd message...");
//     let command = match Command::decode(payload) {
//         Ok(callback) => callback,
//         Err(reason)  => { return Err(JobError::CommandDecodeError{ key, err: reason }); }
//     };
//     let kind = match CommandKind::from_i32(command.kind) {
//         Some(kind) => kind,
//         None       => { return Err(JobError::IllegalCommandKind{ kind: command.kind }); }
//     };

//     // Ignore unkown commands, as we can't dispatch it.
//     if kind == CommandKind::Unknown {
//         warn!("Received UNKOWN command (key: {}); ignoring message", key);
//         return Ok(vec![]);
//     }

//     info!("Received {} command (key: {}).", kind, key);
//     debug!("{:?}", command);

//     // Dispatch command message to appropriate handlers.
//     match kind {
//         CommandKind::Create => {
//             debug!("Handling CREATE command...");
//             cmd_create::handle(debug, &key, command, infra, secrets, xenon_endpoint, xenon_schedulers).await
//         }
//         CommandKind::Stop => unimplemented!(),
//         CommandKind::Unknown => unreachable!(),
//     }
// }
// /*******/
