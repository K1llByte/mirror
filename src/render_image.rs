use std::sync::Arc;

use glam::Vec3;

pub struct RenderImage {
    extent: (usize, usize),
    data: Vec<f32>,
}

// Number of samples the color has
const NUM_SAMPLES: usize = 3;

impl RenderImage {
    pub fn new(extent: (usize, usize)) -> Self {
        Self {
            extent,
            data: vec![0.0; extent.0 * extent.1 * 3],
        }
    }

    pub fn size(&self) -> (usize, usize) {
        self.extent
    }

    pub fn width(&self) -> usize {
        self.extent.0
    }

    pub fn height(&self) -> usize {
        self.extent.1
    }

    pub fn aspect_ratio(&self) -> f32 {
        (self.width() as f32) / (self.height() as f32)
    }

    pub fn get(&self, x: usize, y: usize) -> Vec3 {
        Vec3 {
            x: self.data[y * self.extent.0 * NUM_SAMPLES + x * NUM_SAMPLES + 0],
            y: self.data[y * self.extent.0 * NUM_SAMPLES + x * NUM_SAMPLES + 1],
            z: self.data[y * self.extent.0 * NUM_SAMPLES + x * NUM_SAMPLES + 2],
        }
    }

    pub fn set(&mut self, x: usize, y: usize, value: Vec3) {
        assert!(
            x < self.extent.0 && y < self.extent.1,
            "Invalid pixel coordinates ({}, {})",
            x,
            y
        );

        self.data[y * self.extent.0 * NUM_SAMPLES + x * NUM_SAMPLES + 0] = value.x.clamp(0.0, 1.0);
        self.data[y * self.extent.0 * NUM_SAMPLES + x * NUM_SAMPLES + 1] = value.y.clamp(0.0, 1.0);
        self.data[y * self.extent.0 * NUM_SAMPLES + x * NUM_SAMPLES + 2] = value.z.clamp(0.0, 1.0);
    }

    pub fn to_bytes(&self) -> Arc<[u8]> {
        // Render image has luminance data, values
        let linear_to_gamma = |v: &f32| v.sqrt().max(0.0);
        self.data
            .iter()
            .map(linear_to_gamma)
            .map(|v| (v * 255.0) as u8)
            .collect()
    }
}
