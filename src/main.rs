use std::io;

use clap::Parser;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tracing::{debug, error, info, warn};

use crate::config::Config;
use crate::packet::{MirrorPacket, PacketError};

mod config;
mod packet;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to config toml file
    #[arg(short, long)]
    config: Option<String>,
}

async fn work_task(mut socket: TcpStream) {
    let address = socket.peer_addr().unwrap();
    info!("Connected to '{}'", address);

    MirrorPacket::Ping.write(&mut socket).await.unwrap();
    debug!("Sent ping!");

    loop {
        match MirrorPacket::read(&mut socket).await {
            Ok(MirrorPacket::Ping) => {
                debug!("Received Ping!");
            }
            Err(PacketError::UnkownError) => {
                error!("Protocol error");
                return;
            }
            Err(PacketError::Io(error)) if error.kind() == io::ErrorKind::UnexpectedEof => {
                info!("Disconnected from '{}'", address);
                return;
            }
            Err(error) => {
                error!("IoError: {error}");
                return;
            }
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load command line arguments.
    let args = Args::try_parse()?;

    // Initialize logger.
    tracing_subscriber::fmt()
        .with_env_filter("mirror=trace")
        .init();

    // Load config file if specified, otherwise use default.
    let config = match &args.config {
        Some(path) => {
            info!("Loaded config from '{}'", path);
            Config::from_file(&path).await?
        }
        None => {
            info!("Using default config");
            Default::default()
        }
    };

    // Listen to incoming connections.
    let listener = TcpListener::bind(&config.host).await?;
    info!("Server listening on {}", &config.host);

    // Connect to bootstrap peers.
    info!("Connecting to bootstrap peers");
    for peer_address in &config.bootstrap_peers {
        let Ok(socket) = TcpStream::connect(peer_address).await else {
            warn!("Could not connect to bootstrap {}", peer_address);
            continue;
        };
        // Dispatch into a separate task.
        tokio::spawn(work_task(socket));
    }

    loop {
        // Handle incoming connections.
        let (socket, _) = listener.accept().await?;

        // Dispatch into a separate task.
        tokio::spawn(work_task(socket));
    }
}
