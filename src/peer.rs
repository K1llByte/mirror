use std::net::SocketAddr;
use std::sync::Arc;
use std::{collections::HashMap, io};

use core::future::Future;
use tokio::net::TcpStream;
use tokio::net::tcp::OwnedWriteHalf;
use tokio::sync::Mutex;
use tracing::{debug, error, info, trace, warn};

use crate::packet::{MirrorPacket, PacketError};

pub type PeerTable = Arc<Mutex<HashMap<SocketAddr, Peer>>>;

#[derive(Debug)]
pub struct Peer {
    // pub write_socket: OwnedWriteHalf,
    pub write_socket: OwnedWriteHalf,
}

async fn peer_task_impl(peer_table: PeerTable, socket: TcpStream, listen_port: u16) -> () {
    let local_listen_address = socket.local_addr().unwrap();
    let peer_address = socket.peer_addr().unwrap();
    info!("Connected to '{}'", peer_address);
    let (mut read_socket, mut write_socket) = socket.into_split();

    // 1. Send Hello packet with the listening port of this peer.
    MirrorPacket::Hello(listen_port)
        .write(&mut write_socket)
        .await
        .unwrap();

    // 2. Receive Hello packet from remote peer.
    let peer_listen_port = match MirrorPacket::read(&mut read_socket).await {
        Ok(MirrorPacket::Hello(peer_listen_port)) => peer_listen_port,
        _ => {
            error!("Unexpected protocol behaviour. Closing connection.");
            return;
        }
    };
    let peer_listen_address = SocketAddr::new(peer_address.ip(), peer_listen_port);

    {
        let mut peer_table_guard = peer_table.lock().await;
        if peer_table_guard.contains_key(&peer_listen_address) {
            info!("Already connected to {peer_listen_address}. Closing connection.");
            return;
        }

        // 3. Register peer into the routing table
        peer_table_guard.insert(
            peer_listen_address,
            Peer {
                write_socket: write_socket,
            },
        );
        debug!(
            "PeerTable connections: {:?}",
            peer_table_guard
                .iter()
                .map(|(k, v)| (v.write_socket.peer_addr().unwrap().port(), k.port()))
                .collect::<Vec<_>>()
        );
        let peer = peer_table_guard
            .get_mut(&peer_listen_address)
            .expect("Unexpected, this entry was just inserted");

        // // 4. Send known peers.
        // let peer_vec = peer_table.lock().await.keys().map(Clone::clone).collect();
        // MirrorPacket::GossipPeers(peer_vec)
        //     .write(&mut peer.write_socket)
        //     .await
        //     .unwrap();
    }

    // 5. Proceed with normal flow.
    'outer: loop {
        match MirrorPacket::read(&mut read_socket).await {
            Ok(MirrorPacket::Hello(_)) => {
                // Whilst the remote peer is connected, it's unexpected for it
                // to change its listening port.
                warn!("Unexpected Hello packet.");
                continue;
            }
            Ok(MirrorPacket::GossipPeers(new_peers)) => {
                info!(
                    "{} requested to connect to {:?}",
                    peer_listen_port, new_peers
                );
                // // For each new peer, try to create connection.
                // for peer_address in new_peers {
                //     if peer_table
                //         .lock()
                //         .await
                //         .keys()
                //         .any(|addr| *addr == peer_address || *addr == local_listen_address)
                //     {
                //         trace!("Will not connect to {peer_address}");
                //         continue;
                //     }
                //     trace!("Connecting to {peer_address} ...");

                //     let Ok(new_socket) = TcpStream::connect(peer_address).await else {
                //         warn!("Could not connect to peer {}", peer_address);
                //         continue;
                //     };
                //     // Dispatch into a separate task.
                //     tokio::spawn(peer_task(peer_table.clone(), new_socket, listen_port));
                // }
            }
            Err(PacketError::Io(error)) if error.kind() == io::ErrorKind::UnexpectedEof => {
                debug!("Doing to disconnect from '{}'", peer_address);
                break 'outer;
            }
            Err(error) => {
                error!("IoError: {error}");
                break 'outer;
            }
        }
    }
}

pub fn peer_task(
    peer_table: PeerTable,
    socket: TcpStream,
    listen_port: u16,
) -> impl Future<Output = ()> + Send {
    async move {
        let peer_address = socket.peer_addr().unwrap();
        peer_task_impl(peer_table.clone(), socket, listen_port).await;
        info!("Removing from table '{}'", peer_address);
        // peer_table.lock().await.g;
        info!("Disconnected from '{}'", peer_address);
    }
}
