use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use clap::Parser;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use tracing::{info, trace, warn};

use crate::config::Config;
use crate::peer::{Peer, peer_task};

mod config;
mod packet;
mod peer;
// mod renderer;
// mod scene;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to config toml file
    #[arg(short, long)]
    config: Option<String>,
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

    let peer_table = Arc::new(Mutex::new(HashMap::<SocketAddr, Peer>::new()));

    // Listen to incoming connections.
    let listener = TcpListener::bind(&config.host).await?;
    let listen_port = listener.local_addr()?.port();
    info!("Server listening on {}", &config.host);
    info!("Port: {}", listen_port);

    // Connect to bootstrap peers.
    info!("Connecting to bootstrap peers");
    for peer_address in &config.bootstrap_peers {
        let Ok(socket) = TcpStream::connect(peer_address).await else {
            warn!("Could not connect to bootstrap peer {}", peer_address);
            continue;
        };
        // Dispatch into a separate task.
        tokio::spawn(peer_task(peer_table.clone(), socket, listen_port));
    }

    loop {
        // Handle incoming connections.
        let (socket, _) = listener.accept().await?;

        // Dispatch into a separate task.
        tokio::spawn(peer_task(peer_table.clone(), socket, listen_port));
    }
}
