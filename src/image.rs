use std::sync::Arc;

use bincode::{Decode, Encode};
use glam::Vec3;

pub type Tile = Image;

#[derive(Debug, Encode, Decode)]
pub struct Image {
    extent: (usize, usize),
    data: Box<[f32]>,
}

// Number of samples the color has
const NUM_PIXEL_SAMPLES: usize = 3;

impl Image {
    pub fn new(extent: (usize, usize)) -> Self {
        assert!(extent.0 > 0 && extent.1 > 0, "Invalid image size");
        Self {
            extent,
            data: vec![0.0; extent.0 * extent.1 * NUM_PIXEL_SAMPLES].into_boxed_slice(),
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
            x: self.data[y * self.extent.0 * NUM_PIXEL_SAMPLES + x * NUM_PIXEL_SAMPLES + 0],
            y: self.data[y * self.extent.0 * NUM_PIXEL_SAMPLES + x * NUM_PIXEL_SAMPLES + 1],
            z: self.data[y * self.extent.0 * NUM_PIXEL_SAMPLES + x * NUM_PIXEL_SAMPLES + 2],
        }
    }

    pub fn set(&mut self, x: usize, y: usize, value: Vec3) {
        assert!(
            x < self.extent.0 && y < self.extent.1,
            "Invalid pixel coordinates ({}, {})",
            x,
            y
        );

        self.data[y * self.extent.0 * NUM_PIXEL_SAMPLES + x * NUM_PIXEL_SAMPLES + 0] =
            value.x.clamp(0.0, 1.0);
        self.data[y * self.extent.0 * NUM_PIXEL_SAMPLES + x * NUM_PIXEL_SAMPLES + 1] =
            value.y.clamp(0.0, 1.0);
        self.data[y * self.extent.0 * NUM_PIXEL_SAMPLES + x * NUM_PIXEL_SAMPLES + 2] =
            value.z.clamp(0.0, 1.0);
    }

    pub fn insert_tile(&mut self, tile: &Tile, pos: (usize, usize)) {
        assert!(
            pos.0 + tile.size().0 <= self.size().0 && pos.1 + tile.size().1 <= self.size().1,
            "Invalid image tile insertion"
        );
        for ty in 0..tile.height() {
            for tx in 0..tile.width() {
                self.set(pos.0 + tx, pos.1 + ty, tile.get(tx, ty));
            }
        }
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
