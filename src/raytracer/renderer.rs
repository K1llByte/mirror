use crate::raytracer::{Hittable, Ray, Scene, Tile};

use glam::Vec3;
use rand::{Rng, SeedableRng, rngs::SmallRng};

pub struct Renderer {
    max_bounces: usize,
}

impl Renderer {
    pub fn new() -> Self {
        Self { max_bounces: 50 }
    }

    pub fn trace(&self, scene: &Scene, ray: &Ray, depth: usize) -> Vec3 {
        // Depth is the maximum number of recursive ray bounces possible
        if depth == 0 {
            return Vec3::ZERO;
        }

        let Some(hit) = scene.hit(&ray) else {
            return scene.background();
        };

        let Some(scattered) = hit.material.scatter(ray, &hit) else {
            return hit.material.emission();
        };

        let scattering = scattered.attenuation * self.trace(scene, &scattered.ray, depth - 1);
        scattering + hit.material.emission()
    }

    pub fn render_tile(
        &self,
        scene: &Scene,
        samples_per_pixel: usize,
        begin_pos: (usize, usize),
        tile_size: (usize, usize),
        image_size: (usize, usize),
    ) -> Tile {
        let mut tile = Tile::new(tile_size);
        let mut rng = SmallRng::from_rng(&mut rand::rng());

        let sample_weight = 1.0 / (samples_per_pixel as f32);
        for v in 0..tile_size.1 {
            for u in 0..tile_size.0 {
                let mut pixel_color = Vec3::ZERO;
                // Ray trace for each sample
                for _ in 0..samples_per_pixel {
                    let sample_u = (2.0 * (u + begin_pos.0) as f32 / image_size.0 as f32) - 1.0
                        + rng.random_range(0.0..(2.0 / image_size.0 as f32));
                    let sample_v = (2.0 * (v + begin_pos.1) as f32 / image_size.1 as f32) - 1.0
                        + rng.random_range(0.0..(2.0 / image_size.1 as f32));

                    // Trace pixel color
                    let ray = scene.camera().create_viewport_ray(sample_u, sample_v);
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
