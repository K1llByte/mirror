use core::f32;

use bincode::{Decode, Encode};
use glam::Vec3;

use crate::raytracer::Ray;

pub trait Intersectable {
    fn intersect(&self, ray: &Ray) -> bool;
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct Aabb {
    #[bincode(with_serde)]
    pub min_position: Vec3,
    #[bincode(with_serde)]
    pub max_position: Vec3,
}

impl Aabb {
    pub fn empty() -> Self {
        Self {
            min_position: Vec3::INFINITY,
            max_position: Vec3::NEG_INFINITY,
        }
    }

    pub fn new(position: Vec3, size: Vec3) -> Self {
        assert!(
            size.x > 0.0 && size.y > 0.0 && size.z > 0.0,
            "Size of Aabb must be positive"
        );
        let half_size = size / 2.0;
        Self {
            min_position: position - half_size,
            max_position: position + half_size,
        }
    }

    pub fn from_positions(min_position: Vec3, max_position: Vec3) -> Self {
        Self {
            min_position,
            max_position,
        }
    }

    pub fn surround(aabb1: &Aabb, aabb2: &Aabb) -> Self {
        Self {
            min_position: aabb1.min_position.min(aabb2.min_position),
            max_position: aabb1.max_position.max(aabb2.max_position),
        }
    }
}

impl Intersectable for Aabb {
    fn intersect(&self, ray: &Ray) -> bool {
        let inv_dir = ray.direction().map(|d| {
            if d.abs() < f32::MIN_POSITIVE {
                f32::MAX
            } else {
                1.0 / d
            }
        });

        let t0 = (self.min_position - ray.origin()) * inv_dir;
        let t1 = (self.max_position - ray.origin()) * inv_dir;
        let t_min = Vec3::min(t0, t1);
        let t_max = Vec3::max(t0, t1);
        let t_enter = t_min.max_element();
        let t_exit = t_max.min_element();
        t_enter <= t_exit && t_exit >= 0.0
    }
}
