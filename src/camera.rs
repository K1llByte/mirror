use bincode::{Decode, Encode};
use glam::Vec3;

use crate::ray::Ray;

#[derive(Debug, Clone, Encode, Decode)]
pub struct Camera {
    #[bincode(with_serde)]
    position: Vec3,
    width: f32,
    height: f32,
}

impl Camera {
    pub fn new(position: Vec3, width: f32, height: f32) -> Self {
        Self {
            position,
            width,
            height,
        }
    }

    pub fn position(&self) -> Vec3 {
        self.position
    }
    pub fn width(&self) -> f32 {
        self.width
    }
    pub fn height(&self) -> f32 {
        self.height
    }
    pub fn aspect_ratio(&self) -> f32 {
        self.width / self.height
    }

    // Both u and v must be within [0, width] and [0, height].
    pub fn create_viewport_ray(&self, u: f32, v: f32) -> Ray {
        // TODO: Change camera implementation to use fov and aspect_ratio
        // instead of image sizes.
        let focal_length = 1.0;
        let viewport_height = 2.0;
        let viewport_width = viewport_height * (self.width() / self.height());

        let viewport_u = Vec3::new(viewport_width, 0.0, 0.0);
        let viewport_v = Vec3::new(0.0, -viewport_height, 0.0);

        let pixel_delta_u = viewport_u / self.width();
        let pixel_delta_v = viewport_v / self.height();

        let viewport_upper_left = self.position()
            - Vec3::new(0.0, 0.0, focal_length)
            - viewport_u / 2.0
            - viewport_v / 2.0;
        let pixel00_loc = viewport_upper_left + 0.5 * (pixel_delta_u + pixel_delta_v);

        let pixel_center = pixel00_loc + (u * pixel_delta_u) + (v * pixel_delta_v);
        Ray::new(self.position, pixel_center - self.position)
    }
}
