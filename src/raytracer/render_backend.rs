#[cfg(not(target_arch = "wasm32"))]
use crate::protocol::{MirrorPacket, PeerTable};
use crate::raytracer::{AccumulatedImage, Renderer, Scene, Tile};
use crate::utils;

use async_channel::{Receiver, Sender, TryRecvError};
use bincode::{Decode, Encode};
use futures::future;
use std::{
    cmp::{max, min},
    net::SocketAddr,
    sync::{
        Arc,
        atomic::{self, AtomicUsize},
    },
    time::Instant,
};
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tracing::{error, info, trace};

#[derive(Clone)]
pub struct RenderBackend {
    pub renderer: Arc<Renderer>,
    #[cfg(not(target_arch = "wasm32"))]
    pub peer_table: PeerTable,
}

#[derive(Debug, Clone, Copy, Encode, Decode)]
pub struct TileRenderWork {
    pub begin_pos: (usize, usize),
    pub tile_size: (usize, usize),
}

async fn local_render_tile_task(
    work_send_queue: Sender<TileRenderWork>,
    work_recv_queue: Receiver<TileRenderWork>,
    remaining_tiles: Arc<AtomicUsize>,
    render_backend: RenderBackend,
    render_image: Arc<RwLock<AccumulatedImage>>,
    scene: Arc<Scene>,
    samples_per_pixel: usize,
) {
    let mut rendered_tiles = Vec::new();

    // Do render work until theres no more
    let (image_size, times_sampled) = {
        let image_render_guard = render_image.read().await;
        (image_render_guard.size(), image_render_guard.times_sampled)
    };
    loop {
        // Receive work
        // info!("About to receive render work");
        if let Ok(tile_render_work) = work_recv_queue.recv().await {
            // Do work
            let tile = render_backend.renderer.render_tile(
                &scene,
                samples_per_pixel,
                tile_render_work.begin_pos,
                tile_render_work.tile_size,
                image_size,
            );
            rendered_tiles.push((tile_render_work.begin_pos, tile));
            // Decrement number of remainder tiles to be rendered and close
            // shared send queue to signal other tasks to end work.
            if remaining_tiles.fetch_sub(1, atomic::Ordering::Relaxed) <= 1 {
                work_send_queue.close();
            }
        } else {
            break;
        }
    }

    // Insert result tiles in render_image
    {
        let total_samples = samples_per_pixel + times_sampled;
        let sampled_weight = times_sampled as f32 / total_samples as f32;
        let new_sample_weight = (samples_per_pixel as f32) / (total_samples as f32);
        let mut image_guard = render_image.write().await;
        for (begin_pos, tile) in rendered_tiles {
            image_guard.insert_tile_by(&tile, begin_pos, |c, n| {
                c * sampled_weight + n * new_sample_weight
            });
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
async fn remote_render_tile_task(
    work_send_queue: Sender<TileRenderWork>,
    work_recv_queue: Receiver<TileRenderWork>,
    remaining_tiles: Arc<AtomicUsize>,
    render_backend: RenderBackend,
    render_image: Arc<RwLock<AccumulatedImage>>,
    scene: Arc<Scene>,
    peer_listen_address: SocketAddr,
    samples_per_pixel: usize,
) {
    let render_batch_size: usize = 8;
    let mut render_batch = Vec::with_capacity(render_batch_size);
    let mut accum_roudtrip_time: u128 = 0;
    let mut accum_rendering_time: u128 = 0;

    let mut rendered_tiles = Vec::new();

    let (image_size, times_sampled) = {
        let image_render_guard = render_image.read().await;
        (image_render_guard.size(), image_render_guard.times_sampled)
    };

    // Synchronize scene before requesting to render tiles
    {
        let mut peer_table_guard = render_backend.peer_table.write().await;
        let peer = peer_table_guard
            .get_mut(&peer_listen_address)
            .expect("Peer data should exist");
        // FIXME: We shouldn't need to clone when we want to send the scene.
        if let Err(_) = (MirrorPacket::SyncScene((*scene).clone()))
            .write(&mut peer.write_socket)
            .await
        {
            error!("Remote work task failed to send render tile work");
            return;
        }
    }

    // Do render work until there's no more
    'outer: loop {
        // Receive work
        if let Ok(tile_render_work) = work_recv_queue.recv().await {
            render_batch.push(tile_render_work);
            // Drain up to render_batch_size-1 additional items without waiting.
            while render_batch.len() < render_batch_size {
                match work_recv_queue.try_recv() {
                    Ok(work) => render_batch.push(work),
                    Err(TryRecvError::Closed) => break 'outer,
                    Err(TryRecvError::Empty) => break,
                }
            }

            // Do work
            let tiles = {
                let roundtrip_timer = Instant::now();
                let tile_recv_queue = {
                    let mut peer_table_guard = render_backend.peer_table.write().await;
                    let peer = peer_table_guard
                        .get_mut(&peer_listen_address)
                        .expect("Peer data should exist");
                    // Send render request
                    trace!("Sending a render batch with {} tiles", render_batch.len());
                    if let Err(_) = (MirrorPacket::RenderTileRequest {
                        tiles: render_batch.clone(),
                        image_size,
                        samples_per_pixel,
                    })
                    .write(&mut peer.write_socket)
                    .await
                    {
                        error!("Remote work task failed to send render tile work");
                        // Reinsert work back into the channel
                        for work in render_batch.iter() {
                            work_send_queue.send(work.clone()).await.unwrap();
                        }
                        break;
                    }
                    // trace!("Time sending request: {} ms", timer.elapsed().as_millis());
                    peer.tile_recv_queue.clone()
                };

                // Receive render response
                let (tiles, render_time) = match tile_recv_queue.recv().await {
                    Ok(response) => response,
                    Err(_) => {
                        error!("Unexpected receiver queue error");
                        // Reinsert work back into the channel
                        for work in render_batch.iter() {
                            work_send_queue.send(work.clone()).await.unwrap();
                        }
                        break;
                    }
                };

                let roundtrip_time = roundtrip_timer.elapsed().as_millis();
                accum_rendering_time += render_time;
                accum_roudtrip_time += roundtrip_time;
                tiles
            };

            for (work, tile) in render_batch.iter().zip(tiles) {
                rendered_tiles.push((work.begin_pos, tile));
            }

            // Decrement number of remainder tiles to be rendered and close
            // channel so other tasks can finish and join.
            if remaining_tiles.fetch_sub(render_batch.len(), atomic::Ordering::Relaxed)
                <= render_batch.len()
            {
                work_send_queue.close();
            }
            render_batch.clear();
        } else {
            break;
        }
    }

    // Insert result tiles in render_image
    {
        let total_samples = samples_per_pixel + times_sampled;
        let sampled_weight = times_sampled as f32 / total_samples as f32;
        let new_sample_weight = (samples_per_pixel as f32) / (total_samples as f32);
        let mut image_guard = render_image.write().await;
        for (begin_pos, tile) in rendered_tiles.iter() {
            image_guard.insert_tile_by(&tile, *begin_pos, |c, n| {
                c * sampled_weight + n * new_sample_weight
            });
        }
    }

    let average_roudtrip_time = accum_roudtrip_time as f32 / rendered_tiles.len() as f32;
    let average_rendering_time = accum_rendering_time as f32 / rendered_tiles.len() as f32;
    let average_latency_time = average_roudtrip_time - average_rendering_time;
    trace!("Rendered tiles: {}", rendered_tiles.len());
    trace!(
        "Average roundtrip time (rendering + latency): {} ms",
        average_roudtrip_time
    );
    trace!("Average rendering time: {} ms", average_rendering_time);
    trace!("Average latency time: {} ms", average_latency_time);
    trace!("Total roundtrip time {} ms", accum_roudtrip_time);
    trace!("Total rendering time {} ms", accum_rendering_time);
}

/// Render info struct with render timings. Every time value is measured in
/// milliseconds.
pub struct RenderInfo {
    pub total_samples: usize,
    pub total_time: u128,
    pub last_samples: usize,
    pub last_time: u128,
    pub total_avg_time_per_sample: u128,
    pub last_avg_time_per_sample: u128,
}

impl RenderInfo {
    pub fn merge(&mut self, new: &RenderInfo) {
        self.total_avg_time_per_sample =
            (self.total_time + new.total_time) / (self.total_samples + new.total_samples) as u128;
        self.total_avg_time_per_sample =
            (self.last_time + new.last_time) / (self.last_samples + new.last_samples) as u128;
        self.total_samples += new.total_samples;
        self.total_time += new.total_time;
        self.last_samples = new.last_samples;
        self.last_time = new.last_time;
    }
}

impl Default for RenderInfo {
    fn default() -> Self {
        Self {
            total_samples: 0,
            total_time: 0,
            last_samples: 0,
            last_time: 0,
            total_avg_time_per_sample: 0,
            last_avg_time_per_sample: 0,
        }
    }
}

pub async fn render_task(
    render_backend: RenderBackend,
    render_image: Arc<RwLock<AccumulatedImage>>,
    scene: Arc<Scene>,
    samples_per_pixel: usize,
) -> RenderInfo {
    // Measure execution time from here
    let render_start = utils::instant_now();

    const RENDER_TILE_MAX_SIZE: (usize, usize) = (64, 64);
    let image_size = render_image.read().await.size();
    assert!(image_size.0 >= RENDER_TILE_MAX_SIZE.0 && image_size.1 >= RENDER_TILE_MAX_SIZE.1);

    let num_width_tiles = image_size.0 / RENDER_TILE_MAX_SIZE.0
        + (image_size.0 % RENDER_TILE_MAX_SIZE.0 != 0) as usize;
    let num_height_tiles = image_size.1 / RENDER_TILE_MAX_SIZE.1
        + (image_size.1 % RENDER_TILE_MAX_SIZE.1 != 0) as usize;
    let remaining_tiles = Arc::new(AtomicUsize::new(num_height_tiles * num_width_tiles));

    let (work_send_queue, work_recv_queue) = async_channel::unbounded::<TileRenderWork>();

    let num_remote_tasks = {
        #[cfg(not(target_arch = "wasm32"))]
        {
            render_backend.peer_table.read().await.len()
        }
        #[cfg(target_arch = "wasm32")]
        {
            0
        }
    };
    let num_processors = utils::ideal_processors();
    let num_local_tasks = max(
        num_processors - min(num_remote_tasks, num_processors / 2),
        1,
    );

    let mut local_handles = Vec::with_capacity(num_local_tasks);
    let mut remote_handles: Vec<JoinHandle<()>> = Vec::with_capacity(num_remote_tasks);

    // Dispatch work tasks:
    // - Local render_tile tasks: An amount of CPU cores.
    for _ in 0..num_local_tasks {
        local_handles.push(utils::spawn(local_render_tile_task(
            work_send_queue.clone(),
            work_recv_queue.clone(),
            remaining_tiles.clone(),
            render_backend.clone(),
            render_image.clone(),
            scene.clone(),
            samples_per_pixel,
        )));
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        // - Remote render_tile tasks: As many as connected peers.
        for peer_listen_address in render_backend.peer_table.read().await.keys().cloned() {
            remote_handles.push(utils::spawn(remote_render_tile_task(
                work_send_queue.clone(),
                work_recv_queue.clone(),
                remaining_tiles.clone(),
                render_backend.clone(),
                render_image.clone(),
                scene.clone(),
                peer_listen_address,
                samples_per_pixel,
            )));
        }
    }

    // Loop over all tiles splitted to be rendered. This loop takes into
    // account the last remainder tiles that could not be of size
    // RENDER_TILE_MAX_SIZE.
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

    // Join all work task handles
    future::join(
        future::join_all(local_handles),
        future::join_all(remote_handles),
    )
    .await;

    {
        render_image.write().await.times_sampled += samples_per_pixel;
    }

    // Log render time
    let render_time = (utils::instant_now() - render_start) as u128;
    info!(
        "Rendered {} sample(s) in {} ms",
        samples_per_pixel, render_time
    );

    let total_avg_time_per_sample = render_time / samples_per_pixel as u128;
    RenderInfo {
        total_samples: samples_per_pixel,
        total_time: render_time,
        last_samples: samples_per_pixel,
        last_time: render_time,
        total_avg_time_per_sample,
        last_avg_time_per_sample: total_avg_time_per_sample,
    }
}
