use bincode::{Decode, Encode};
use glam::Vec3;

#[derive(Debug, Clone, Encode, Decode)]
pub struct Sphere {
    #[bincode(with_serde)]
    pub position: Vec3,
    pub radius: f32,
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct Camera {
    #[bincode(with_serde)]
    pub position: Vec3,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct Scene {
    pub camera: Camera,
    pub objects: Vec<Sphere>,
}
