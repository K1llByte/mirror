use std::io;
use std::net::SocketAddr;

use bincode::error::{DecodeError, EncodeError};
use bincode::{Decode, Encode, config, decode_from_slice};
use thiserror::Error;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::raytracer::{Scene, Tile, TileRenderWork};

/// Represents the main control packet used in the peer-to-peer network.
#[derive(Debug, Encode, Decode)]
pub enum MirrorPacket {
    /// Initial 'hello' handshake packet type, sent during the initial
    /// handshake phase to inform a peer of the sender’s name and active
    /// listening port. This port can then be shared with other peers to help
    /// them join the network.
    Hello(Option<String>, u16),
    /// Gossip protocol packet type, used to distribute a list of known peer
    /// socket addresses, helping peers build and maintain an up-to-date view
    /// of the network.
    GossipPeers(Vec<SocketAddr>),
    /// Scene synchronization packet type, used to synchronize scene between
    /// useful network peers before RenderTileRequest.
    SyncScene(Scene),
    /// Tile render request packet type, used to request peer to render tile
    /// packet.
    RenderTileRequest {
        tiles: Vec<TileRenderWork>,
        image_size: (usize, usize),
        samples_per_pixel: usize,
    },
    /// Tile render response packet type, response oof the RenderTileRequest
    /// packet type.
    RenderTileResponse { tiles: Vec<Tile>, render_time: u128 },
}

#[derive(Debug, Error)]
pub enum PacketError {
    #[error("{0}")]
    Io(#[from] io::Error),
    #[error("{0}")]
    Decode(#[from] DecodeError),
    #[error("{0}")]
    Encode(#[from] EncodeError),
}

impl MirrorPacket {
    pub async fn read<S: AsyncRead + Unpin>(stream: &mut S) -> Result<Self, PacketError> {
        // Read 4-byte length prefix
        let mut len_buf = [0u8; 4];
        stream.read_exact(&mut len_buf).await?;
        let len = u32::from_be_bytes(len_buf) as usize;
        // Read the binary data
        let mut bytes = vec![0u8; len];
        stream.read_exact(&mut bytes).await?;

        let (packet, _): (MirrorPacket, usize) = decode_from_slice(&bytes, config::standard())?;

        Ok(packet)
    }

    pub async fn write<S: AsyncWrite + Unpin>(&self, stream: &mut S) -> Result<(), PacketError> {
        let serialized = bincode::encode_to_vec(&self, config::standard())?;

        // Send length as u32
        let len_bytes = (serialized.len() as u32).to_be_bytes();
        stream.write_all(&len_bytes).await?;
        stream.write_all(&serialized).await?;
        stream.flush().await?;

        Ok(())
    }
}
