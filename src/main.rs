use clap::Parser;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tracing::{error, info, warn};

use crate::config::Config;

mod config;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to config toml file
    #[arg(short, long)]
    config: Option<String>,
}

async fn work_task(mut socket: TcpStream) {
    let address = socket.peer_addr().unwrap();
    let mut buf = [0u8; 1024];

    loop {
        match socket.read(&mut buf).await {
            Ok(0) => {
                info!("Peer {} disconnected", address);
                return;
            }
            Ok(n) => {
                info!("Pong from {}", address);
                // Echo the message back to the client.
                if let Err(e) = socket.write_all(&buf[..n]).await {
                    error!("Failed to write to socket: {}", e);
                    return;
                }
            }
            Err(e) => {
                error!("Failed to read from socket: {}", e);
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
        let (socket, address) = listener.accept().await?;
        info!("New connection: {}", address);

        // Dispatch into a separate task.
        tokio::spawn(work_task(socket));
    }
}
