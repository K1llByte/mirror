use std::sync::Arc;

use glam::Vec3;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Sphere {
    pub position: Vec3,
    pub radius: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Camera {
    pub position: Vec3,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Scene {
    pub camera: Camera,
    pub objects: Vec<Sphere>,
}
