use std::time::Duration;

use tokio::time::sleep;
use tracing::debug;

use crate::peer::PeerTable;

pub struct Renderer {
    pub peer_table: PeerTable,
}

impl Renderer {
    pub fn new(pt: PeerTable) -> Self {
        Self { peer_table: pt }
    }
}
pub async fn render(/* , &Scene */) -> () {
    debug!("Before sleep");
    sleep(Duration::from_secs(1)).await;
    debug!("Called render in an async context")
}
