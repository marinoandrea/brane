use anyhow::{Context, Result};
use brane_net::interface::{NetEvent, NetEventKind};
use clap::Clap;
use dotenv::dotenv;
use log::{LevelFilter, debug, warn};
use prost::{bytes::BytesMut, Message};
use rdkafka::{
    admin::{AdminClient, AdminOptions, NewTopic, TopicReplication},
    error::RDKafkaErrorCode,
    producer::{FutureProducer, FutureRecord},
    util::Timeout,
    ClientConfig,
    message::ToBytes,
};
use socksx::socks6::{self, SocksReply};
use tokio::net::{TcpListener, TcpStream};

#[derive(Clap)]
#[clap(version = env!("CARGO_PKG_VERSION"))]
struct Opts {
    #[clap(short, long, default_value = "0.0.0.0:5081", env = "ADDRESS")]
    /// Service address
    address: String,
    /// Kafka brokers
    #[clap(short, long, default_value = "localhost:9092", env = "BROKERS")]
    brokers: String,
    /// Topic to send callbacks to
    #[clap(short, long = "evt-topic", env = "EVENT_TOPIC")]
    event_topic: String,
    /// Print debug info
    #[clap(short, long, env = "DEBUG", takes_value = false)]
    debug: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
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

    // Ensure that the callback topic (output) exists.
    let event_topic = opts.event_topic.clone();
    ensure_event_topic(&event_topic, &opts.brokers).await?;

    let producer: FutureProducer = ClientConfig::new()
        .set("bootstrap.servers", &opts.brokers)
        .set("message.timeout.ms", "5000")
        .create()
        .context("Failed to create Kafka producer.")?;

    // Start listening for SOCKS connections.
    let listener = TcpListener::bind(opts.address).await?;
    loop {
        let (socket, _) = listener.accept().await?;

        let producer = producer.clone();
        let event_topic = event_topic.clone();

        tokio::spawn(async move { handle_connection(socket, producer, event_topic).await });
    }
}

///
///
///
pub async fn handle_connection(
    mut source: TcpStream,
    producer: FutureProducer,
    event_topic: String,
) -> Result<()> {
    match socks6::read_request(&mut source).await {
        Ok(request) => {
            socks6::no_authentication(&mut source).await?;

            if let Ok(mut destination) = TcpStream::connect(request.destination.to_string()).await {
                // EVENT: connection has been established between source and destination.
                emit_event(NetEventKind::Connected, &producer, &event_topic, None).await?;

                socks6::write_reply(&mut source, socks6::SocksReply::Success).await?;
                socks6::write_initial_data(&mut destination, &request).await?;

                // Patch together the source and destination sockets, collect number of bytes transfered.
                let (_bytes_sd, _bytes_ds) = socksx::copy_bidirectional(&mut source, &mut destination).await?;

                // EVENT: connection has been closed between source and destination.
                emit_event(NetEventKind::Disconnected, &producer, &event_topic, None).await?;
            } else {
                warn!("host unreachable");
                socks6::write_reply(&mut source, SocksReply::HostUnreachable).await?;
            }
        }
        Err(_) => {
            warn!("general failure");
            socks6::write_reply(&mut source, SocksReply::GeneralFailure).await?;
        }
    }

    Ok(())
}

///
///
///
pub async fn emit_event(
    kind: NetEventKind,
    producer: &FutureProducer,
    event_topic: &str,
    payload: Option<Vec<u8>>,
) -> Result<()> {
    // Get metadata from SOCKS options.
    let application = "app".to_string();
    let location = "loc".to_string();
    let job_id = "job".to_string();
    let order = 1;

    // Create new event.
    let event_key = format!("{}#{}", job_id, order);
    let event = NetEvent::new(
        kind,
        application,
        location,
        job_id,
        order,
        payload,
        None,
    );

    // Encode event as bytes.
    let mut payload = BytesMut::with_capacity(64);
    event.encode(&mut payload).unwrap();

    // Send event on output topic
    let message = FutureRecord::to(&event_topic).key(&event_key).payload(payload.to_bytes());

    if let Err(error) = producer.send(message, Timeout::Never).await {
        log::error!("Failed to send event (key: {}): {:?}", event_key, error);
    }

    Ok(())
}

///
///
///
pub async fn ensure_event_topic(
    event_topic: &str,
    brokers: &str,
) -> Result<()> {
    let admin_client: AdminClient<_> = ClientConfig::new()
        .set("bootstrap.servers", brokers)
        .set("message.timeout.ms", "5000")
        .create()
        .context("Failed to create Kafka admin client.")?;

    let results = admin_client
        .create_topics(
            &[NewTopic::new(event_topic, 1, TopicReplication::Fixed(1))],
            &AdminOptions::new(),
        )
        .await?;

    // Report on the results. Don't consider 'TopicAlreadyExists' an error.
    for result in results {
        match result {
            Ok(topic) => log::info!("Kafka topic '{}' created.", topic),
            Err((topic, error)) => match error {
                RDKafkaErrorCode::TopicAlreadyExists => {
                    log::info!("Kafka topic '{}' already exists", topic);
                }
                _ => {
                    anyhow::bail!("Kafka topic '{}' not created: {:?}", topic, error);
                }
            },
        }
    }

    Ok(())
}