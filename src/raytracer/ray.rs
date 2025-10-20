use core::f32;

use glam::Vec3;

#[derive(Debug, Clone)]
pub struct Ray {
    origin: Vec3,
    direction: Vec3,
    tmin: f32,
    tmax: f32,
}

impl Ray {
    pub const MIN_RAY_DISTANCE: f32 = 0.001;
    pub const MAX_RAY_DISTANCE: f32 = f32::MAX;

    pub fn new(origin: Vec3, direction: Vec3) -> Self {
        if !direction.is_normalized() {
            println!("direction: {}", direction);
        }
        assert!(
            direction.is_normalized(),
            "Ray direction must be normalized"
        );
        Self {
            origin,
            direction,
            tmin: Self::MIN_RAY_DISTANCE,
            tmax: Self::MAX_RAY_DISTANCE,
        }
    }

    pub fn origin(&self) -> Vec3 {
        self.origin
    }

    pub fn direction(&self) -> Vec3 {
        self.direction
    }

    pub fn tmin(&self) -> f32 {
        self.tmin
    }

    pub fn tmax(&self) -> f32 {
        self.tmax
    }

    /// Compute ray position at a certain t.
    pub fn at(&self, t: f32) -> Vec3 {
        self.origin + t * self.direction
    }

    /// Creates a new ray with updated tmax. If new tmax is lesser than current
    /// tmax then this function returns a clone of this ray.
    pub fn with_tmax(&self, new_tmax: f32) -> Ray {
        let mut ray = self.clone();
        ray.tmax = new_tmax.min(self.tmax);
        ray
    }

    /// Creates a new ray with updated tmin. If new tmin is greater that
    /// current tmin then this function returns a clone of this ray.
    pub fn with_tmin(&self, new_tmin: f32) -> Ray {
        let mut ray = self.clone();
        ray.tmin = new_tmin.max(self.tmin);
        ray
    }
}
