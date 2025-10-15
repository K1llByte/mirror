use std::fmt::Display;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::{collections::HashMap, io};

use async_channel::Receiver;
use core::future::Future;
use tokio::net::tcp::OwnedWriteHalf;
use tokio::net::{TcpListener, TcpStream, ToSocketAddrs};
use tokio::sync::RwLock;
use tokio::task::{self};
use tokio::time;
use tracing::{debug, error, info, trace, warn};

use crate::protocol::{MirrorPacket, PacketError};
use crate::raytracer::{Renderer, Scene, Tile};

pub type PeerTable = Arc<RwLock<HashMap<SocketAddr, Peer>>>;

#[derive(Debug)]
pub struct Peer {
    pub name: Option<String>,
    pub write_socket: OwnedWriteHalf,
    pub tile_recv_queue: Receiver<Tile>,
}

/// Listen task, responsible for connecting to bootstrap peers and handling new
/// incomming connections and spawning new peer tasks.
pub async fn listen_task(
    renderer: Arc<Renderer>,
    host: impl ToSocketAddrs + Display,
    bootstrap_peers: Vec<SocketAddr>,
) -> io::Result<()> {
    // Bind listener address
    let listener = TcpListener::bind(&host).await?;
    let listen_port = listener.local_addr()?.port();
    info!("Server listening on {}", &host);

    // Connect to bootstrap peers.
    info!("Connecting to bootstrap peers ...");
    connect_to_peers(bootstrap_peers, renderer.clone(), listen_port, "Bootstrap").await;

    loop {
        // Handle incoming connections.
        let (socket, _) = listener.accept().await?;
        // Dispatch into a separate task.
        tokio::spawn(peer_task(renderer.clone(), socket, listen_port, "Listen"));
    }
}

pub async fn connect_to_peers<P: IntoIterator<Item = impl Into<SocketAddr>>>(
    peers: P,
    renderer: Arc<Renderer>,
    listen_port: u16,
    tag: &'static str,
) {
    // TODO: Do the trick of spawning multiple tasks at once and join them immediatelly
    for peer_listen_address in peers {
        let peer_listen_address = peer_listen_address.into();
        // FIXME: Hardcoded 127.0.0.1 for now, will
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
        if renderer
            .peer_table
            .read()
            .await
            .contains_key(&peer_listen_address)
        {
            warn!(
                "[{tag}, {:?}] - Trying to connect to duplicate peer '{peer_listen_address}'. Skipped.",
                task::try_id()
            );
            continue;
        }

        // Proceed with connection
        let timeout_duration = Duration::from_secs(5);
        let Ok(Ok(socket)) =
            time::timeout(timeout_duration, TcpStream::connect(&peer_listen_address)).await
        else {
            warn!(
                "[{tag}, {:?}] - Could not connect to peer '{peer_listen_address}'",
                task::try_id(),
            );
            continue;
        };
        // Dispatch into a separate task.
        tokio::spawn(peer_task(renderer.clone(), socket, listen_port, tag));
    }
}

pub fn peer_task(
    renderer: Arc<Renderer>,
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

        let (tile_send_queue, tile_recv_queue) = async_channel::unbounded();
        {
            let mut peer_table_guard = renderer.peer_table.write().await;
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
                    tile_recv_queue,
                },
            );
            // Once its added to the peer table, its considered connected to the network.
            trace!("[{tag}, {}] - Connected to '{}'", task::id(), peer_address);
            let peer_vec = peer_table_guard
                .keys()
                .filter(|&pa| *pa != peer_listen_address)
                .cloned()
                .collect();
            let peer = peer_table_guard
                .get_mut(&peer_listen_address)
                .expect("Unexpected, this entry was just inserted");

            // 4. Send known peers.
            MirrorPacket::GossipPeers(peer_vec)
                .write(&mut peer.write_socket)
                .await
                .unwrap()
        }

        let mut scene: Option<Scene> = None;

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
                    connect_to_peers(new_peers, renderer.clone(), listen_port, "Gossip").await;
                }
                Ok(MirrorPacket::SyncScene(received_scene)) => {
                    debug!(
                        "[{tag}, {}] Received scene: {:?}",
                        task::id(),
                        received_scene
                    );
                    scene = Some(received_scene);
                }
                Ok(MirrorPacket::RenderTileRequest {
                    begin_pos,
                    tile_size,
                    image_size,
                    samples_per_pixel,
                }) => {
                    let timer = Instant::now();
                    debug!("[{tag}, {}] Received render tile request", task::id());
                    if scene.is_none() {
                        warn!(
                            "[{tag}, {}] Scene was not synchronized before render request. Ignoring ...",
                            task::id()
                        );
                        continue;
                    }
                    let tile = renderer.render_tile(
                        scene.as_ref().unwrap(),
                        samples_per_pixel,
                        begin_pos,
                        tile_size,
                        image_size,
                    );

                    let mut peer_table_guard = renderer.peer_table.write().await;
                    let peer = peer_table_guard
                        .get_mut(&peer_listen_address)
                        .expect("Should be available while this tasks runs");
                    if let Err(err) = MirrorPacket::RenderTileResponse(tile)
                        .write(&mut peer.write_socket)
                        .await
                    {
                        error!("[{tag}, {}] Error: {:?}", task::id(), err);
                    }
                    trace!(
                        "Time spent rendering for another peer: {} ms",
                        timer.elapsed().as_millis()
                    );
                }
                Ok(MirrorPacket::RenderTileResponse(tile)) => {
                    debug!("[{tag}, {}] Received render tile response", task::id());
                    if let Err(err) = tile_send_queue.send(tile).await {
                        error!("{err}")
                    }
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
            }
        }

        renderer
            .peer_table
            .write()
            .await
            .remove(&peer_listen_address);
        info!(
            "[{tag}, {}] - Disconnected from '{}'",
            task::id(),
            peer_address
        );
    }
}
