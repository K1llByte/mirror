use std::collections::HashMap;
use std::io;
use std::net::SocketAddr;
use std::sync::Arc;

use clap::Parser;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

use crate::config::Config;
use crate::packet::{MirrorPacket, PacketError};
use crate::peer::{Peer, PeerTable};

mod config;
mod packet;
mod peer;
mod renderer;
mod scene;

async fn peer_task(peer_table: PeerTable, socket: TcpStream, listen_port: u16) -> () {
    let peer_address = socket.peer_addr().unwrap();
    info!("Connected to '{}'", peer_address);
    let (mut read_socket, mut write_socket) = socket.into_split();

    // 1. Send Hello packet with the listening port of this peer.
    MirrorPacket::Hello(listen_port)
        .write(&mut write_socket)
        .await
        .unwrap();
    debug!("Sent Hello({listen_port})!");

    // 2. Receive Hello packet from remote peer.
    let peer_listen_port = match MirrorPacket::read(&mut read_socket).await {
        Ok(MirrorPacket::Hello(peer_listen_port)) => {
            debug!("Received Hello({peer_listen_port})!");
            peer_listen_port
        }
        _ => {
            error!("Unexpected protocol behaviour. Closing connection.");
            return;
        }
    };

    // 3. Send known peers.
    let peer_vec = peer_table
        .lock()
        .await
        .values()
        .map(|peer| peer.listen_addr)
        .collect();
    debug!("PeerVec: {:?}", peer_vec);
    MirrorPacket::GossipPeers(peer_vec)
        .write(&mut write_socket)
        .await
        .unwrap();

    // 3. Register peer into the routing table
    let listen_addr = SocketAddr::new(peer_address.ip(), peer_listen_port);
    peer_table.lock().await.insert(
        peer_address,
        Peer {
            write_socket,
            listen_addr,
        },
    );
    debug!("PeerTable: {:?}", peer_table.lock().await.keys());

    // 5. Proceed with normal flow.
    loop {
        match MirrorPacket::read(&mut read_socket).await {
            Ok(MirrorPacket::Hello(_)) => {
                // Whilst the remote peer is connected, it's unexpected for it
                // to change its listening port.
                warn!("Unexpected Hello packet.");
                continue;
            }
            Ok(MirrorPacket::GossipPeers(new_peers)) => {
                info!("Peer wants us to connect to {:?}", new_peers);
                // For each new peer, try to create connection.
                for peer_address in new_peers {
                    let Ok(new_socket) = TcpStream::connect(peer_address).await else {
                        warn!("Could not connect to peer {}", peer_address);
                        continue;
                    };
                    // Dispatch into a separate task.
                    // tokio::spawn(async move {
                    //     peer_task(peer_table.clone(), new_socket, listen_port).await
                    // });
                }

                info!("Added new peers")
            }
            Err(PacketError::Io(error)) if error.kind() == io::ErrorKind::UnexpectedEof => {
                info!("Disconnected from '{}'", peer_address);
                return;
            }
            Err(error) => {
                error!("IoError: {error}");
                return;
            }
        }
    }
}

fn assert_send_sync<T: Send + Sync>(_: T) {}
fn assert_send<T: Send>(_: T) {}
fn assert_sync<T: Sync>(_: T) {}

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
