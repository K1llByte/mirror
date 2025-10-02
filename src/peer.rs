use std::fmt::Display;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use std::{collections::HashMap, io};

use core::future::Future;
use glam::Vec3;
use tokio::net::tcp::OwnedWriteHalf;
use tokio::net::{TcpListener, TcpStream, ToSocketAddrs};
use tokio::sync::Mutex;
use tokio::task::{self};
use tracing::{debug, error, info, trace, warn};

use crate::packet::{MirrorPacket, PacketError};
use crate::scene::{Camera, Scene, Sphere};

pub type PeerTable = Arc<Mutex<HashMap<SocketAddr, Peer>>>;

#[derive(Debug)]
pub struct Peer {
    pub write_socket: OwnedWriteHalf,
    pub name: Option<String>,
}

pub async fn listen_task(
    peer_table: PeerTable,
    host: impl ToSocketAddrs + Display,
    bootstrap_peers: Vec<SocketAddr>,
) -> io::Result<()> {
    // Bind listener address
    let listener = TcpListener::bind(&host).await?;
    let listen_port = listener.local_addr()?.port();
    info!("Server listening on {}", &host);

    // Connect to bootstrap peers.
    info!("Connecting to bootstrap peers ...");
    connect_to_peers(
        bootstrap_peers,
        peer_table.clone(),
        listen_port,
        "Bootstrap",
    )
    .await;

    loop {
        // Handle incoming connections.
        let (socket, _) = listener.accept().await?;

        // Dispatch into a separate task.
        tokio::spawn(peer_task(peer_table.clone(), socket, listen_port, "Listen"));
    }
}

pub async fn connect_to_peers<P: IntoIterator<Item = impl Into<SocketAddr>>>(
    peers: P,
    peer_table: PeerTable,
    listen_port: u16,
    tag: &'static str,
) {
    for peer_listen_address in peers {
        let peer_listen_address = peer_listen_address.into();
        // TODO: Hardcoded 127.0.0.1 for now, will
        let local_listen_address =
            SocketAddr::from_str(format!("127.0.0.1:{listen_port}").as_str()).unwrap();
        // Avoid trying to connect this my peer to itself
        if peer_listen_address == local_listen_address {
            warn!(
                "[{tag}, {:?}] - Trying to connect to self '{peer_listen_address}'. Skipped.",
                task::try_id()
            );
            continue;
        }
        // Refuse duplicate connections
        if peer_table.lock().await.contains_key(&peer_listen_address) {
            warn!(
                "[{tag}, {:?}] - Trying to connect to duplicate peer '{peer_listen_address}'. Skipped.",
                task::try_id()
            );
            continue;
        }

        // Proceed with connection
        let Ok(socket) = TcpStream::connect(&peer_listen_address).await else {
            warn!(
                "[{tag}, {:?}] - Could not connect to peer '{peer_listen_address}'",
                task::try_id(),
            );
            continue;
        };
        // Dispatch into a separate task.
        tokio::spawn(peer_task(peer_table.clone(), socket, listen_port, tag));
    }
}

pub async fn peer_write_task(peer_table: PeerTable, peer_listen_address: SocketAddr) {
    const SECS: u64 = 5;
    for s in 0..SECS {
        tokio::time::sleep(Duration::from_secs(1)).await;
        debug!("Sending scene in {}", SECS - s);
    }
    debug!("HELLO WORLD!");

    let sphere_left = Sphere {
        position: Vec3::new(-1.0, 0.0, -1.0),
        radius: 0.5,
    };
    let sphere_center = Sphere {
        position: Vec3::new(0.0, 0.0, -1.0),
        radius: 0.5,
    };
    let sphere_right = Sphere {
        position: Vec3::new(1.0, 0.0, -1.0),
        radius: 0.5,
    };
    let sphere_ground = Sphere {
        position: Vec3::new(0.0, -100.5, -1.0),
        radius: 100.0,
    };

    // Scene
    let scene = Scene {
        camera: Camera {
            position: Vec3::ZERO,
            width: 400f32,
            height: 300f32,
        },
        objects: vec![sphere_left, sphere_center, sphere_right, sphere_ground],
    };

    let mut peer_table_guard = peer_table.lock().await;
    let Some(peer) = peer_table_guard.get_mut(&peer_listen_address) else {
        return;
    };
    MirrorPacket::SyncScene(scene)
        .write(&mut peer.write_socket)
        .await
        .expect("This is supposed to work since im just testing");
}

pub fn peer_task(
    peer_table: PeerTable,
    socket: TcpStream,
    listen_port: u16,
    tag: &'static str,
) -> impl Future<Output = ()> + Send {
    async move {
        let local_listen_address = socket.local_addr().unwrap();
        let peer_address = socket.peer_addr().unwrap();
        trace!(
            "[{tag}, {}] - Starting handshake with '{}'",
            task::id(),
            peer_address
        );
        let (mut read_socket, mut write_socket) = socket.into_split();

        // 1. Send Hello packet with the listening port of this peer.
        MirrorPacket::Hello(None, listen_port)
            .write(&mut write_socket)
            .await
            .unwrap();

        // 2. Receive Hello packet from remote peer.
        let (peer_name, peer_listen_port) = match MirrorPacket::read(&mut read_socket).await {
            Ok(MirrorPacket::Hello(peer_name, peer_listen_port)) => (peer_name, peer_listen_port),
            _ => {
                error!(
                    "[{tag}, {}] - Unexpected protocol behaviour. Refused handshake.",
                    task::id()
                );
                return;
            }
        };
        let peer_listen_address = SocketAddr::new(peer_address.ip(), peer_listen_port);

        {
            let mut peer_table_guard = peer_table.lock().await;
            // Refuse self connections
            if peer_listen_address == local_listen_address {
                info!(
                    "[{tag}, {}] - Trying to connect to self '{peer_listen_address}'. Refused handshake.",
                    task::id()
                );
                return;
            }
            // Refuse duplicate connections
            if peer_table_guard.contains_key(&peer_listen_address) {
                info!(
                    "[{tag}, {}] - Already connected to '{peer_listen_address}'. Refused handshake.",
                    task::id()
                );
                return;
            }

            // 3. Register peer into the peer table
            peer_table_guard.insert(
                peer_listen_address,
                Peer {
                    name: peer_name,
                    write_socket,
                },
            );
            // Once its added to the peer table, its considered connected to the network.
            trace!("[{tag}, {}] - Connected to '{}'", task::id(), peer_address);
            let peer_vec = peer_table_guard
                .keys()
                .filter(|&pa| *pa != peer_listen_address)
                .cloned()
                .collect();
            debug!(
                "[{tag}, {}] - PeerTable connections: {:?}",
                task::id(),
                peer_table_guard
                    .iter()
                    .map(|(k, v)| (v.write_socket.peer_addr().unwrap().port(), k.port()))
                    .collect::<Vec<_>>()
            );
            let peer = peer_table_guard
                .get_mut(&peer_listen_address)
                .expect("Unexpected, this entry was just inserted");

            // 4. Send known peers.
            MirrorPacket::GossipPeers(peer_vec)
                .write(&mut peer.write_socket)
                .await
                .unwrap()
        }

        // FIXME: Temporary
        // tokio::spawn(peer_write_task(peer_table.clone(), peer_listen_address));

        // 5. Proceed with normal flow.
        'outer: loop {
            match MirrorPacket::read(&mut read_socket).await {
                Ok(MirrorPacket::Hello(_, _)) => {
                    // Whilst the remote peer is connected, it's unexpected for it
                    // to change its listening port.
                    warn!("[{tag}, {}] - Unexpected Hello packet.", task::id());
                    continue;
                }
                Ok(MirrorPacket::GossipPeers(new_peers)) => {
                    info!(
                        "[{tag}, {}] - {} requested to connect to {:?}",
                        task::id(),
                        peer_listen_port,
                        new_peers
                    );
                    connect_to_peers(new_peers, peer_table.clone(), listen_port, "Gossip").await;
                    debug!(
                        "[{tag}, {}] PeerTable connections: {:?}",
                        task::id(),
                        peer_table
                            .lock()
                            .await
                            .iter()
                            .map(|(k, v)| (v.write_socket.peer_addr().unwrap().port(), k.port()))
                            .collect::<Vec<_>>()
                    );
                }
                Ok(MirrorPacket::SyncScene(scene)) => {
                    debug!("[{tag}, {}] Received scene: {:?}", task::id(), scene);
                }
                Err(PacketError::Io(error)) if error.kind() == io::ErrorKind::UnexpectedEof => {
                    debug!(
                        "[{tag}, {}] - Going to disconnect from '{}'",
                        task::id(),
                        peer_address
                    );
                    break 'outer;
                }
                Err(error) => {
                    error!("[{tag}, {}] - IoError: {error}", task::id());
                    break 'outer;
                }
                _ => {}
            }
        }

        peer_table.lock().await.remove(&peer_listen_address);
        info!(
            "[{tag}, {}] - Disconnected from '{}'",
            task::id(),
            peer_address
        );

        debug!(
            "[{tag}, {}] PeerTable connections: {:?}",
            task::id(),
            peer_table
                .lock()
                .await
                .iter()
                .map(|(k, v)| (v.write_socket.peer_addr().unwrap().port(), k.port()))
                .collect::<Vec<_>>()
        );
    }
}
