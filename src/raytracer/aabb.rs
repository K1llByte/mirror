use core::f32;

use glam::Vec3;

use crate::raytracer::Ray;

pub trait Intersectable {
    fn intersect(&self, ray: &Ray) -> bool;
}

pub struct Aabb {
    min_position: Vec3,
    max_position: Vec3,
}

impl Aabb {
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
