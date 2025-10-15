use bincode::{Decode, Encode};
use glam::Vec3;

use crate::raytracer::Ray;

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
