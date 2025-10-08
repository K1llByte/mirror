use std::{
    cmp::min,
    net::SocketAddr,
    num::NonZero,
    sync::{
        Arc,
        atomic::{self, AtomicUsize},
    },
    thread,
    time::Instant,
};

use async_channel::Receiver;
use futures::future;
use glam::Vec3;
use rand::{Rng, SeedableRng, rngs::SmallRng};
use tokio::sync::Mutex;
use tracing::{error, info};

use crate::{
    image::{Image, Tile},
    packet::MirrorPacket,
    peer::PeerTable,
    ray::Ray,
    scene::{Hittable, Scene},
};

pub struct Renderer {
    pub peer_table: PeerTable,
    samples_per_pixel: usize,
    times_sampled: AtomicUsize,
    max_bounces: usize,
}

impl Renderer {
    pub fn new(pt: PeerTable) -> Self {
        Self {
            peer_table: pt,
            samples_per_pixel: 4,
            max_bounces: 10,
            times_sampled: AtomicUsize::new(0),
        }
    }

    pub fn samples_per_pixel(&self) -> usize {
        self.samples_per_pixel
    }

    pub fn times_sampled(&self) -> usize {
        self.times_sampled.load(atomic::Ordering::Acquire)
    }

    pub fn update_times_sampled(&self) {
        self.times_sampled
            .fetch_add(self.samples_per_pixel, atomic::Ordering::Acquire);
    }

    pub fn trace(&self, scene: &Scene, ray: &Ray, depth: usize) -> Vec3 {
        // Depth is the maximum number of recursive ray bounces possible
        if depth == 0 {
            return Vec3::new(0.0, 0.0, 0.0);
        }

        if let Some(hit) = scene.hit(&ray) {
            if let Some(scattered) = hit.material.scatter(ray, &hit) {
                return scattered.attenuation * self.trace(scene, &scattered.ray, depth - 1);
            }
            return Vec3::new(0.2, 0.2, 0.2);
        }

        let a = 0.5 * (ray.direction().normalize().y + 1.0);
        (1.0 - a) * Vec3::new(1.0, 1.0, 1.0) + a * Vec3::new(0.5, 0.7, 1.0)
    }

    pub fn render_tile(
        &self,
        scene: &Scene,
        begin_pos: (usize, usize),
        tile_size: (usize, usize),
        image_size: (usize, usize),
    ) -> Tile {
        let mut tile = Tile::new(tile_size);
        let mut rng = SmallRng::from_rng(&mut rand::rng());

        let sample_weight = 1.0 / (self.samples_per_pixel as f32);
        for v in 0..tile_size.1 {
            for u in 0..tile_size.0 {
                let mut pixel_color = Vec3::ZERO;
                // Ray trace for each sample
                for _ in 0..self.samples_per_pixel {
                    let sample_u = (2.0 * (u + begin_pos.0) as f32 / image_size.0 as f32) - 1.0
                        + rng.random_range(0.0..(2.0 / image_size.0 as f32));
                    let sample_v = (2.0 * (v + begin_pos.1) as f32 / image_size.1 as f32) - 1.0
                        + rng.random_range(0.0..(2.0 / image_size.1 as f32));

                    // Trace pixel color
                    let ray = scene.camera.create_viewport_ray(sample_u, sample_v);
                    let sample_color = self.trace(&scene, &ray, self.max_bounces);

                    pixel_color += sample_color * sample_weight;
                }
                // Ray trace for this pixel
                tile.set(u, v, pixel_color);
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
    scene: Arc<Scene>,
) {
    let total_samples = renderer.samples_per_pixel() + renderer.times_sampled();
    let sampled_weight = renderer.times_sampled() as f32 / total_samples as f32;
    let new_sample_weight = 1.0 / (total_samples as f32);

    let image_size = render_image.lock().await.size();
    loop {
        // Receive work
        if let Ok(tile_render_work) = work_recv_queue.recv().await {
            // Do work
            let tile = renderer.render_tile(
                &scene,
                tile_render_work.begin_pos,
                tile_render_work.tile_size,
                image_size,
            );
            // Insert result tile in render_image
            render_image
                .lock()
                .await
                .insert_tile_by(&tile, tile_render_work.begin_pos, |c, n| {
                    c * sampled_weight + n * new_sample_weight
                });
        } else {
            break;
        }
    }
}

async fn remote_render_tile_task(
    work_recv_queue: Receiver<TileRenderWork>,
    renderer: Arc<Renderer>,
    render_image: Arc<Mutex<Image>>,
    scene: Arc<Scene>,
    peer_listen_address: SocketAddr,
) {
    let total_samples = renderer.samples_per_pixel() + renderer.times_sampled();
    let sampled_weight = renderer.times_sampled() as f32 / total_samples as f32;
    let new_sample_weight = 1.0 / (total_samples as f32);

    let image_size = render_image.lock().await.size();

    // Synchronize scene before requesting to render tiles
    {
        let mut peer_table_guard = renderer.peer_table.lock().await;
        let peer = peer_table_guard
            .get_mut(&peer_listen_address)
            .expect("Peer data should exist");
        // FIXME: We shouldn't need to clone when we want to send the scene.
        if let Err(_) = (MirrorPacket::SyncScene((*scene).clone()))
            .write(&mut peer.write_socket)
            .await
        {
            error!("Remote work task failed to send render tile work");
            todo!("Fault tolerance: if fails to send, do something.");
        }
    }

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
                if let Err(_) = (MirrorPacket::RenderTileRequest {
                    begin_pos: tile_render_work.begin_pos,
                    tile_size: tile_render_work.tile_size,
                    image_size,
                })
                .write(&mut peer.write_socket)
                .await
                {
                    error!("Remote work task failed to send render tile work");
                    todo!("Fault tolerance: if fails to send, do something.");
                }

                // Receive render response
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
                .insert_tile_by(&tile, tile_render_work.begin_pos, |c, n| {
                    c * sampled_weight + n * new_sample_weight
                });
        } else {
            break;
        }
    }
}

pub async fn render_task(
    renderer: Arc<Renderer>,
    render_image: Arc<Mutex<Image>>,
    scene: Arc<Scene>,
) {
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
            scene.clone(),
        )));
    }
    // - Remote render_tile tasks: As many as connected peers.
    for peer_listen_address in renderer.peer_table.lock().await.keys().cloned() {
        join_handles.push(tokio::spawn(remote_render_tile_task(
            work_recv_queue.clone(),
            renderer.clone(),
            render_image.clone(),
            scene.clone(),
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

    renderer.update_times_sampled();

    // Log render time
    info!(
        "Rendered {} sample(s) in {} ms",
        renderer.samples_per_pixel(),
        render_time.elapsed().as_millis()
    );
}
