use std::{
    cmp::min,
    net::SocketAddr,
    num::NonZero,
    sync::Arc,
    thread::{self, sleep},
    time::{Duration, Instant},
};

use async_channel::Receiver;
use futures::future;
use glam::Vec3;
use rand::Rng;
use tokio::sync::Mutex;
use tracing::{debug, error, info};

use crate::{
    image::{Image, Tile},
    packet::MirrorPacket,
    peer::PeerTable,
};

pub struct Renderer {
    pub peer_table: PeerTable,
}

impl Renderer {
    pub fn new(pt: PeerTable) -> Self {
        Self { peer_table: pt }
    }

    pub fn render_tile(&self, _begin: (usize, usize), size: (usize, usize)) -> Tile {
        // let mut res = Vec::with_capacity(size.0 * size.1 * 3);
        let mut tile = Image::new(size);
        let mut rng = rand::rng();
        let random_rbg: [f32; _] = [rng.random(), rng.random(), rng.random()];
        for y in 0..size.1 {
            for x in 0..size.0 {
                tile.set(x, y, Vec3::new(random_rbg[0], random_rbg[1], random_rbg[2]));
            }
        }
        tile
    }
}

struct TileRenderWork {
    pub begin_pos: (usize, usize),
    pub tile_size: (usize, usize),
}

async fn local_render_tile_task(
    work_recv_queue: Receiver<TileRenderWork>,
    renderer: Arc<Renderer>,
    render_image: Arc<Mutex<Image>>,
) {
    loop {
        // Receive work
        if let Ok(tile_render_work) = work_recv_queue.recv().await {
            // Do work
            let tile = renderer.render_tile(tile_render_work.begin_pos, tile_render_work.tile_size);
            // Insert result tile in render_image
            render_image
                .lock()
                .await
                .insert_tile(&tile, tile_render_work.begin_pos);
        } else {
            break;
        }
    }
}

async fn remote_render_tile_task(
    work_recv_queue: Receiver<TileRenderWork>,
    renderer: Arc<Renderer>,
    render_image: Arc<Mutex<Image>>,
    peer_listen_address: SocketAddr,
) {
    loop {
        // Receive work
        if let Ok(tile_render_work) = work_recv_queue.recv().await {
            // Do work
            let tile = {
                let mut peer_table_guard = renderer.peer_table.lock().await;
                let peer = peer_table_guard
                    .get_mut(&peer_listen_address)
                    .expect("Peer data should exist");
                // Send render request
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
                match peer.tile_recv_queue.recv().await {
                    Ok(tile) => tile,
                    Err(_) => {
                        error!("Unexpected receiver queue error");
                        todo!("Fault tolerance: if fails to send, do something.");
                    }
                }
            };

            // Insert result tile in render_image
            render_image
                .lock()
                .await
                .insert_tile(&tile, tile_render_work.begin_pos);
        } else {
            break;
        }
    }
}

pub async fn render_task(renderer: Arc<Renderer>, render_image: Arc<Mutex<Image>>) {
    // Measure execution time from here
    let render_time = Instant::now();

    const RENDER_TILE_MAX_SIZE: (usize, usize) = (64, 64);
    let image_size = render_image.lock().await.size();
    assert!(image_size.0 >= RENDER_TILE_MAX_SIZE.0 && image_size.1 >= RENDER_TILE_MAX_SIZE.1);

    let (work_send_queue, work_recv_queue) = async_channel::unbounded::<TileRenderWork>();

    let num_local_tasks = thread::available_parallelism()
        .map(NonZero::get)
        .unwrap_or(1);
    let num_remote_tasks = renderer.peer_table.lock().await.len();

    let mut join_handles = Vec::with_capacity(num_local_tasks + num_remote_tasks);

    // Dispatch work tasks:
    // - Local render_tile tasks: As many as CPU cores.
    for _ in 0..num_local_tasks {
        join_handles.push(tokio::spawn(local_render_tile_task(
            work_recv_queue.clone(),
            renderer.clone(),
            render_image.clone(),
        )));
    }
    // - Remote render_tile tasks: As many as connected peers.
    for peer_listen_address in renderer.peer_table.lock().await.keys().cloned() {
        join_handles.push(tokio::spawn(remote_render_tile_task(
            work_recv_queue.clone(),
            renderer.clone(),
            render_image.clone(),
            peer_listen_address,
        )));
    }

    // Loop over all tiles splitted to be rendered. This loop takes into
    // account the last remainder tiles that could not be of size
    // RENDER_TILE_MAX_SIZE.
    let num_width_tiles = image_size.0 / RENDER_TILE_MAX_SIZE.0
        + (image_size.0 % RENDER_TILE_MAX_SIZE.0 != 0) as usize;
    let num_height_tiles = image_size.1 / RENDER_TILE_MAX_SIZE.1
        + (image_size.1 % RENDER_TILE_MAX_SIZE.1 != 0) as usize;
    for ty in 0..num_height_tiles {
        let begin_height = ty * RENDER_TILE_MAX_SIZE.1;
        let tile_height = min(RENDER_TILE_MAX_SIZE.1, image_size.1 - begin_height);
        for tx in 0..num_width_tiles {
            let begin_width = tx * RENDER_TILE_MAX_SIZE.0;
            let tile_width = min(RENDER_TILE_MAX_SIZE.0, image_size.0 - begin_width);

            // Send work to queue
            work_send_queue
                .send(TileRenderWork {
                    begin_pos: (begin_width, begin_height),
                    tile_size: (tile_width, tile_height),
                })
                .await
                .unwrap();
        }
    }

    // Close channel so that tasks can finish and join
    work_send_queue.close();

    // Join all work task handles
    future::join_all(join_handles).await;

    // Log render time
    info!("Render time: {} ms", render_time.elapsed().as_millis());
}
