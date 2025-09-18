use std::io;

use thiserror::Error;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

pub enum MirrorPacket {
    Ping,
    // GossipPeers,
}

#[derive(Debug, Error)]
pub enum PacketError {
    #[error("Unkown error")]
    UnkownError,
    #[error("{0}")]
    Io(#[from] io::Error),
}

impl MirrorPacket {
    pub async fn read<S: AsyncRead + Unpin>(stream: &mut S) -> Result<Self, PacketError> {
        let packet_type = stream.read_u8().await?;
        match packet_type {
            0 => Ok(Self::Ping),
            _ => Err(PacketError::UnkownError),
        }
    }

    pub async fn write<S: AsyncWrite + Unpin>(&self, stream: &mut S) -> io::Result<()> {
        match self {
            MirrorPacket::Ping => stream.write_u8(0).await,
        }
    }
}
