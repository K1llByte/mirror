use bincode::{Decode, Encode};
use glam::Vec3;

use crate::ray::Ray;

// #[derive(Debug, Clone, Encode, Decode)]
// pub struct Camera {
//     #[bincode(with_serde)]
//     position: Vec3,
//     width: f32,
//     height: f32,
// }

// impl Camera {
//     pub fn new(position: Vec3, width: f32, height: f32) -> Self {
//         Self {
//             position,
//             width,
//             height,
//         }
//     }

//     pub fn position(&self) -> Vec3 {
//         self.position
//     }
//     pub fn width(&self) -> f32 {
//         self.width
//     }
//     pub fn height(&self) -> f32 {
//         self.height
//     }
//     pub fn aspect_ratio(&self) -> f32 {
//         self.width / self.height
//     }

//     // Both u and v must be within [0, width] and [0, height].
//     pub fn create_viewport_ray(&self, u: f32, v: f32) -> Ray {
//         // TODO: Change camera implementation to use fov and aspect_ratio
//         // instead of image sizes.
//         let focal_length = 1.0;
//         let viewport_height = 2.0;
//         let viewport_width = viewport_height * (self.width() / self.height());

//         let viewport_u = Vec3::new(viewport_width, 0.0, 0.0);
//         let viewport_v = Vec3::new(0.0, -viewport_height, 0.0);

//         let pixel_delta_u = viewport_u / self.width();
//         let pixel_delta_v = viewport_v / self.height();

//         let viewport_upper_left = self.position()
//             - Vec3::new(0.0, 0.0, focal_length)
//             - viewport_u / 2.0
//             - viewport_v / 2.0;
//         let pixel00_loc = viewport_upper_left + 0.5 * (pixel_delta_u + pixel_delta_v);

//         let pixel_center = pixel00_loc + (u * pixel_delta_u) + (v * pixel_delta_v);
//         Ray::new(self.position, pixel_center - self.position)
//     }
// }

#[derive(Debug, Clone, Encode, Decode)]
pub struct Camera {
    #[bincode(with_serde)]
    position: Vec3,
    #[bincode(with_serde)]
    forward: Vec3,
    #[bincode(with_serde)]
    right: Vec3,
    #[bincode(with_serde)]
    up: Vec3,
    fov: f32,
    aspect_ratio: f32,
}

impl Camera {
    pub fn new(position: Vec3, forward: Vec3, world_up: Vec3, fov: f32, aspect_ratio: f32) -> Self {
        assert!(
            forward.is_normalized() && world_up.is_normalized(),
            "Camera vectors must be normalized"
        );
        assert!(
            fov > 0.0 && fov < 180.0,
            "Invalid field of view value range '0 < fov < 180'"
        );
        assert!(
            aspect_ratio > 0.0 && fov < 180.0,
            "Invalid aspect ratio value ('0 < aspect_ratio')"
        );

        let right = forward.cross(world_up);
        Self {
            position,
            forward,
            right,
            // No need to normalize since 'forward' 'right' are already unit
            // vectors.
            up: right.cross(forward),
            fov,
            aspect_ratio,
        }
    }

    /// Camera position.
    pub fn position(&self) -> Vec3 {
        self.position
    }

    /// Camera orientation forward vector.
    pub fn forward(&self) -> Vec3 {
        self.forward
    }

    /// Camera orientation right vector.
    pub fn right(&self) -> Vec3 {
        self.right
    }

    /// Camera orientation up vector.
    pub fn up(&self) -> Vec3 {
        self.up
    }

    /// Vertical field of view in radians.
    pub fn fov(&self) -> f32 {
        self.fov
    }

    /// Camera viewport aspect ratio.
    pub fn aspect_ratio(&self) -> f32 {
        self.aspect_ratio
    }

    /// Create a ray according to the camera orientation and viewport
    /// coordinate. Both u and v must be within [-1, 1].
    pub fn create_viewport_ray(&self, u: f32, v: f32) -> Ray {
        let vfov = (self.fov as f32).to_radians();
        let half_height = (vfov / 2.0).tan();
        let half_width = self.aspect_ratio * half_height;

        let direction = self.forward + self.right * (u * half_width) + self.up * (v * half_height);

        Ray::new(self.position, direction.normalize())
    }
}
