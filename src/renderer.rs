use std::{net::SocketAddr, num::NonZero, sync::Arc, thread, time::Instant};

use async_channel::Receiver;
use futures::future;
use rand::Rng;
use tokio::sync::Mutex;
use tracing::{debug, error, info};

use crate::{packet::MirrorPacket, peer::PeerTable, render_image::RenderImage};

pub type Tile = Vec<u8>;

pub struct Renderer {
    pub peer_table: PeerTable,
    pub render_image: RenderImage,
}

impl Renderer {
    pub fn new(pt: PeerTable) -> Self {
        Self {
            peer_table: pt,
            render_image: RenderImage::new((400, 300)),
        }
    }

    pub fn render_tile(&self, _begin: (usize, usize), size: (usize, usize)) -> Tile {
        let mut res = Vec::with_capacity(size.0 * size.1 * 3);
        let mut rng = rand::rng();
        let random_rbg: [u8; _] = [rng.random(), rng.random(), rng.random()];
        for _ in 0..size.1 {
            for _ in 0..size.0 {
                res.push(random_rbg[0]);
                res.push(random_rbg[1]);
                res.push(random_rbg[2]);
            }
        }

        res
    }
}

async fn image_insert_tile(
    image: &Arc<Mutex<Vec<u8>>>,
    image_size: (usize, usize),
    tile: &Tile,
    begin_pos: (usize, usize),
    tile_size: (usize, usize),
) {
    let image = &mut image.lock().await;
    const NUM_CHANNELS: usize = 3;
    for ty in 0..tile_size.1 {
        for tx in 0..tile_size.0 {
            let x = begin_pos.0 + tx;
            let y = begin_pos.1 + ty;
            image[0 + NUM_CHANNELS * x + NUM_CHANNELS * image_size.0 * y] =
                tile[0 + NUM_CHANNELS * tx + NUM_CHANNELS * tile_size.0 * ty];
            image[1 + NUM_CHANNELS * x + NUM_CHANNELS * image_size.0 * y] =
                tile[1 + NUM_CHANNELS * tx + NUM_CHANNELS * tile_size.0 * ty];
            image[2 + NUM_CHANNELS * x + NUM_CHANNELS * image_size.0 * y] =
                tile[2 + NUM_CHANNELS * tx + NUM_CHANNELS * tile_size.0 * ty];
        }
    }
}

struct TileRenderWork {
    pub begin_pos: (usize, usize),
    pub tile_size: (usize, usize),
}

async fn local_render_tile_task(
    work_recv_queue: Receiver<TileRenderWork>,
    renderer: Arc<Renderer>,
    render_image: Arc<Mutex<Vec<u8>>>,
) {
    const IMAGE_SIZE: (usize, usize) = (400, 300);
    loop {
        // Receive work
        if let Ok(tile_render_work) = work_recv_queue.recv().await {
            // Do work
            let tile = renderer.render_tile(tile_render_work.begin_pos, tile_render_work.tile_size);

            // Insert result tile in render_image
            image_insert_tile(
                &render_image,
                IMAGE_SIZE,
                &tile,
                tile_render_work.begin_pos,
                tile_render_work.tile_size,
            )
            .await;
        } else {
            break;
        }
    }
}

async fn remote_render_tile_task(
    work_recv_queue: Receiver<TileRenderWork>,
    renderer: Arc<Renderer>,
    render_image: Arc<Mutex<Vec<u8>>>,
    peer_listen_address: SocketAddr,
) {
    const IMAGE_SIZE: (usize, usize) = (400, 300);
    loop {
        // Receive work
        debug!("Before receiving TileRenderWork");
        if let Ok(tile_render_work) = work_recv_queue.recv().await {
            // Do work
            let tile = {
                let mut peer_table_guard = renderer.peer_table.lock().await;
                let peer = peer_table_guard
                    .get_mut(&peer_listen_address)
                    .expect("Peer data should exist");
                // Send render request
                debug!("Sending render tile request");
                if let Err(_) = MirrorPacket::RenderTileRequest(
                    tile_render_work.begin_pos,
                    tile_render_work.tile_size,
                )
                .write(&mut peer.write_socket)
                .await
                {
                    error!("Remote work task failed to send render tile work");
                    todo!("Fault tolerance: if fails to send, do something.");
                }

                // Wait for response tile
                debug!("Before receiving Tile from peer");
                match peer.tile_recv_queue.recv().await {
                    Ok(tile) => tile,
                    Err(_) => {
                        error!("Unexpected receiver queue error");
                        todo!("Fault tolerance: if fails to send, do something.");
                    }
                }
            };

            // Insert result tile in render_image
            image_insert_tile(
                &render_image,
                IMAGE_SIZE,
                &tile,
                tile_render_work.begin_pos,
                tile_render_work.tile_size,
            )
            .await;
        } else {
            break;
        }
    }
}

pub async fn render_task(renderer: Arc<Renderer>) -> Vec<u8> {
    // Measure execution time from here
    let render_time = Instant::now();

    const IMAGE_SIZE: (usize, usize) = (400, 300);
    const RENDER_TILE_MAX_SIZE: (usize, usize) = (50, 150 / 2);
    assert!(IMAGE_SIZE.0 >= RENDER_TILE_MAX_SIZE.0 && IMAGE_SIZE.1 >= RENDER_TILE_MAX_SIZE.1);

    let render_image: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::from(
        [0u8; IMAGE_SIZE.0 * IMAGE_SIZE.1 * 3],
    )));

    let (work_send_queue, work_recv_queue) = async_channel::unbounded::<TileRenderWork>();

    let num_local_tasks = thread::available_parallelism()
        .map(NonZero::get)
        .unwrap_or(1);
    let num_remote_tasks = renderer.peer_table.lock().await.len();

    let mut join_handles = Vec::with_capacity(num_local_tasks + num_remote_tasks);

    // Spawn local worker tasks
    for _ in 0..num_local_tasks {
        join_handles.push(tokio::spawn(local_render_tile_task(
            work_recv_queue.clone(),
            renderer.clone(),
            render_image.clone(),
        )));
    }

    // Spawn remote worker tasks
    for peer_listen_address in renderer.peer_table.lock().await.keys().cloned() {
        join_handles.push(tokio::spawn(remote_render_tile_task(
            work_recv_queue.clone(),
            renderer.clone(),
            render_image.clone(),
            peer_listen_address,
        )));
    }

    // Dispatch initial tasks:
    // - Local render_tile tasks: As many as CPU cores.
    // - Remote render_tile tasks: As many as connected peers.
    let mut begin_height: usize = 0;
    while begin_height < IMAGE_SIZE.1 {
        // FIXME: Im pretty sure this is doable with better maths, but Im lazy now
        let tile_height = if (begin_height + RENDER_TILE_MAX_SIZE.1) <= IMAGE_SIZE.1 {
            RENDER_TILE_MAX_SIZE.1
        } else {
            IMAGE_SIZE.1 % RENDER_TILE_MAX_SIZE.1
        };
        let mut begin_width: usize = 0;
        while begin_width < IMAGE_SIZE.0 {
            // FIXME: Im pretty sure this is doable with better maths, but Im lazy now
            let tile_width = if (begin_width + RENDER_TILE_MAX_SIZE.0) <= IMAGE_SIZE.0 {
                RENDER_TILE_MAX_SIZE.0
            } else {
                IMAGE_SIZE.0 % RENDER_TILE_MAX_SIZE.0
            };

            let begin_pos = (begin_width, begin_height);
            let tile_size = (tile_width, tile_height);

            work_send_queue
                .send(TileRenderWork {
                    begin_pos,
                    tile_size,
                })
                .await
                .unwrap();

            begin_width += RENDER_TILE_MAX_SIZE.0;
        }
        begin_height += RENDER_TILE_MAX_SIZE.1;
    }

    // Close channel so that tasks can finish and join
    work_send_queue.close();

    // Join all work task handles
    future::join_all(join_handles).await;

    // Log render time
    info!("Render time: {} ms", render_time.elapsed().as_millis());

    let mut image = render_image.lock().await;
    std::mem::take(&mut *image)
}
