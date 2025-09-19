use crate::peer::PeerTable;

pub struct Renderer {
    pub peer_table: PeerTable,
}

impl Renderer {
    pub fn new(pt: PeerTable) -> Self {
        Self { peer_table: pt }
    }
}
