use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use tokio::net::tcp::OwnedWriteHalf;
use tokio::sync::Mutex;

pub type PeerTable = Arc<Mutex<HashMap<SocketAddr, Peer>>>;

#[derive(Debug)]
pub struct Peer {
    // pub write_socket: OwnedWriteHalf,
    pub write_socket: OwnedWriteHalf,
    pub listen_addr: SocketAddr,
}
