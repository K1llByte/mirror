use std::io;

use bincode::error::{DecodeError, EncodeError};
use bincode::{Decode, Encode, config, decode_from_slice};
use thiserror::Error;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

#[derive(Debug, Encode, Decode)]
pub enum MirrorPacket {
    Hello(u16),
    // GossipPeers(Vec<Ipv4Addr>, Vec<Ipv6Addr>),
    // RenderTileRequest
    // RenderTileResponse
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
        // Serialize to Vec<u8>
        let serialized = bincode::encode_to_vec(&self, config::standard())?;

        // Send length as u32
        let len_bytes = (serialized.len() as u32).to_be_bytes();
        stream.write_all(&len_bytes).await?;

        stream.write_all(&serialized).await?;

        Ok(())
    }
}
