use std::net::SocketAddr;
use std::sync::Arc;
use std::{collections::HashMap, io};

use core::future::Future;
use tokio::net::TcpStream;
use tokio::net::tcp::OwnedWriteHalf;
use tokio::sync::Mutex;
use tokio::task;
use tracing::{debug, error, info, trace, warn};

use crate::packet::{MirrorPacket, PacketError};

pub type PeerTable = Arc<Mutex<HashMap<SocketAddr, Peer>>>;

#[derive(Debug)]
pub struct Peer {
    pub write_socket: OwnedWriteHalf,
}

pub fn peer_task(
    peer_table: PeerTable,
    socket: TcpStream,
    listen_port: u16,
) -> impl Future<Output = ()> + Send {
    async move {
        let local_listen_address = socket.local_addr().unwrap();
        let peer_address = socket.peer_addr().unwrap();
        trace!(
            "{} - Starting handshake with '{}'",
            task::id(),
            peer_address
        );
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
                error!(
                    "{} - Unexpected protocol behaviour. Refused handshake.",
                    task::id()
                );
                return;
            }
        };
        let peer_listen_address = SocketAddr::new(peer_address.ip(), peer_listen_port);

        {
            let mut peer_table_guard = peer_table.lock().await;
            if peer_table_guard.contains_key(&peer_listen_address) {
                info!(
                    "{} - Already connected to {peer_listen_address}. Refused handshake.",
                    task::id()
                );
                return;
            }

            // 3. Register peer into the peer table
            peer_table_guard.insert(
                peer_listen_address,
                Peer {
                    write_socket: write_socket,
                },
            );
            // Once its added to the peer table, its considered connected to the network.
            trace!("{} - Connected to '{}'", task::id(), peer_address);
            let peer_vec = peer_table_guard.keys().cloned().collect();
            debug!(
                "PeerTable connections: {:?}",
                peer_table_guard
                    .iter()
                    .map(|(k, v)| (
                        v.write_socket
                            .peer_addr()
                            .expect(format!("{} - Unexpected", task::id()).as_str())
                            .port(),
                        k.port()
                    ))
                    .collect::<Vec<_>>()
            );
            let peer = peer_table_guard
                .get_mut(&peer_listen_address)
                .expect("Unexpected, this entry was just inserted");

            // 4. Send known peers.
            MirrorPacket::GossipPeers(peer_vec)
                .write(&mut peer.write_socket)
                .await
                .unwrap();
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
                        "{} - {} requested to connect to {:?}",
                        task::id(),
                        peer_listen_port,
                        new_peers
                    );
                    // For each new peer, try to create connection.
                    for peer_address in new_peers {
                        if peer_table
                            .lock()
                            .await
                            .keys()
                            .any(|addr| *addr == peer_address || *addr == local_listen_address)
                        {
                            trace!("{} - Will not connect to {peer_address}", task::id());
                            continue;
                        }
                        trace!("{} - Connecting to {peer_address} ...", task::id());

                        let Ok(new_socket) = TcpStream::connect(peer_address).await else {
                            warn!(
                                "{} - Could not connect to peer {}",
                                task::id(),
                                peer_address
                            );
                            continue;
                        };
                        // Dispatch into a separate task.
                        tokio::spawn(peer_task(peer_table.clone(), new_socket, listen_port));
                    }
                }
                Err(PacketError::Io(error)) if error.kind() == io::ErrorKind::UnexpectedEof => {
                    debug!(
                        "{} - Going to disconnect from '{}'",
                        task::id(),
                        peer_address
                    );
                    break 'outer;
                }
                Err(error) => {
                    error!("{} - IoError: {error}", task::id());
                    break 'outer;
                }
            }
        }

        peer_table.lock().await.remove(&peer_listen_address);
        info!("{} - Disconnected from '{}'", task::id(), peer_address);

        debug!(
            "PeerTable connections: {:?}",
            peer_table
                .lock()
                .await
                .iter()
                .map(|(k, v)| (v.write_socket.peer_addr().unwrap().port(), k.port()))
                .collect::<Vec<_>>()
        );
    }
}
