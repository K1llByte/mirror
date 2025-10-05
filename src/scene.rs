use bincode::{Decode, Encode};
use glam::Vec3;

use crate::camera::Camera;

#[derive(Debug, Clone, Encode, Decode)]
pub struct Sphere {
    #[bincode(with_serde)]
    pub position: Vec3,
    pub radius: f32,
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct Scene {
    pub camera: Camera,
    pub objects: Vec<Sphere>,
}
