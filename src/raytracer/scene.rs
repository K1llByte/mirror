use std::sync::Arc;

use bincode::{Decode, Encode};
use glam::Vec3;
use tracing::debug;

use crate::raytracer::{Aabb, BvhNode, Camera, Intersectable, Material, Ray};

pub struct Hit {
    pub distance: f32,
    pub position: Vec3,
    pub normal: Vec3,
    pub material: Arc<Material>,
    pub is_front_face: bool,
}

impl Hit {}

pub trait Hittable {
    fn hit(&self, ray: &Ray) -> Option<Hit>;
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct Sphere {
    #[bincode(with_serde)]
    pub position: Vec3,
    pub radius: f32,
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct Model {
    pub geometry: Sphere,
    pub material: Arc<Material>,
}

impl Hittable for Model {
    fn hit(&self, ray: &Ray) -> Option<Hit> {
        const MIN_RAY_DISTANCE: f32 = 0.001;

        let oc = self.geometry.position - ray.origin();
        let a = ray.direction().dot(ray.direction());
        let half_b = ray.direction().dot(oc);
        let c = oc.length_squared() - self.geometry.radius * self.geometry.radius;
        let discriminant = half_b * half_b - a * c;

        // Check if first solution is valid
        let mut distance = (half_b - discriminant.sqrt()) / a;
        if distance < MIN_RAY_DISTANCE {
            // Check if second solution is valid
            // Note: its possible this second solution is the same as solution 1
            // in case the discriminant was zero.
            distance = (half_b + discriminant.sqrt()) / a;
            if distance < MIN_RAY_DISTANCE {
                // Both possible solutions are behind camera
                return None;
            }
        }

        if discriminant >= 0.0 {
            let position = ray.at(distance);
            let outward_normal = (position - self.geometry.position) / self.geometry.radius;
            let is_front_face = outward_normal.dot(ray.direction()) <= 0.0;
            let normal = if is_front_face {
                outward_normal
            } else {
                -outward_normal
            };

            Some(Hit {
                distance,
                position,
                normal,
                material: self.material.clone(),
                is_front_face,
            })
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct Scene {
    camera: Camera,
    objects: Vec<Arc<Model>>,
    bvh: BvhNode<Model>,
}

impl Scene {
    pub fn new(camera: Camera, mut objects: Vec<Arc<Model>>) -> Self {
        let bvh = BvhNode::new(&mut objects[..]);
        debug!("BVH depth: {}", bvh.depth());
        Self {
            camera,
            objects,
            bvh,
        }
    }

    pub fn camera(&self) -> &Camera {
        &self.camera
    }

    pub fn objects(&self) -> &[Arc<Model>] {
        &self.objects
    }
}

impl Hittable for Scene {
    fn hit(&self, ray: &Ray) -> Option<Hit> {
        if true {
            // Use vec
            const MAX_RAY_DISTANCE: f32 = 1000.0;
            let mut closest_hit_distance = MAX_RAY_DISTANCE;
            let mut closest_hit = None;
            for sphere in self.objects.iter() {
                if let Some(hit) = sphere.hit(&ray) {
                    if hit.distance < closest_hit_distance {
                        closest_hit_distance = hit.distance;
                        closest_hit = Some(hit);
                    }
                }
            }
            closest_hit
        } else {
            // Use bvh
            self.bvh.hit(&ray)
        }
    }
}
