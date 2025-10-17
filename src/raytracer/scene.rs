use std::sync::Arc;

use bincode::{Decode, Encode};
use glam::Vec3;
use tracing::{debug, warn};

use crate::raytracer::{Aabb, Bounded, BvhNode, Camera, Intersectable, Material, Ray};

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

////////////////////////////////////////////////////////////////////////////////
// Model
////////////////////////////////////////////////////////////////////////////////

// #[derive(Debug, Clone, Encode, Decode)]
// pub struct Sphere {
//     #[bincode(with_serde)]
//     pub position: Vec3,
//     pub radius: f32,
// }

// #[derive(Debug, Clone, Encode, Decode)]
// pub struct Quad {
//     #[bincode(with_serde)]
//     pub position: Vec3,
//     #[bincode(with_serde)]
//     pub u: Vec3,
//     #[bincode(with_serde)]
//     pub v: Vec3,
// }

#[derive(Debug, Clone, Encode, Decode)]
pub enum Geometry {
    Sphere {
        #[bincode(with_serde)]
        position: Vec3,
        radius: f32,
    },
    Quad {
        #[bincode(with_serde)]
        position: Vec3,
        #[bincode(with_serde)]
        u: Vec3,
        #[bincode(with_serde)]
        v: Vec3,
    },
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct Model {
    pub geometry: Geometry,
    pub material: Arc<Material>,
}

impl Model {
    pub fn new(geometry: Geometry, material: Arc<Material>) -> Self {
        Self { geometry, material }
    }
}

impl Hittable for Model {
    fn hit(&self, ray: &Ray) -> Option<Hit> {
        match self.geometry {
            Geometry::Sphere { position, radius } => {
                let oc = position - ray.origin();
                let a = ray.direction().dot(ray.direction());
                let half_b = ray.direction().dot(oc);
                let c = oc.length_squared() - radius * radius;
                let discriminant = half_b * half_b - a * c;

                // Check if first solution is valid
                let mut distance = (half_b - discriminant.sqrt()) / a;
                if distance < ray.tmin() || distance > ray.tmax() {
                    // Check if second solution is valid
                    // Note: its possible this second solution is the same as solution 1
                    // in case the discriminant was zero.
                    distance = (half_b + discriminant.sqrt()) / a;
                    if distance < ray.tmin() || distance > ray.tmax() {
                        // Both possible solutions are behind camera
                        return None;
                    }
                }

                if discriminant >= 0.0 {
                    let intersection = ray.at(distance);
                    let outward_normal = (intersection - position) / radius;
                    let is_front_face = outward_normal.dot(ray.direction()) <= 0.0;
                    let normal = if is_front_face {
                        outward_normal
                    } else {
                        -outward_normal
                    };

                    Some(Hit {
                        distance,
                        position: intersection,
                        normal,
                        material: self.material.clone(),
                        is_front_face,
                    })
                } else {
                    None
                }
            }
            Geometry::Quad { position, u, v } => {
                // NOTE: These values can be cached in Quad
                let n = u.cross(v);
                let normal = n.normalize();
                let d = normal.dot(position);
                let w = n / n.dot(n);

                let denom = normal.dot(ray.direction());
                // Check if ray is parallel to quad plane
                if denom.abs() < f32::MIN_POSITIVE {
                    return None;
                }

                let distance = (d - normal.dot(ray.origin())) / denom;
                // Check if intersection is within acceptable ray interval
                if distance < ray.tmin() || distance > ray.tmax() {
                    return None;
                }
                let intersection = ray.at(distance);
                let plain_hit_vector = intersection - position;
                let alpha = w.dot(plain_hit_vector.cross(v));
                let beta = w.dot(u.cross(plain_hit_vector));

                if alpha > 1.0 || alpha < 0.0 || beta > 1.0 || beta < 0.0 {
                    return None;
                }

                Some(Hit {
                    distance,
                    position: intersection,
                    normal,
                    material: self.material.clone(),
                    is_front_face: ray.direction().dot(normal) < 0.0,
                })
            }
        }
    }
}

impl Bounded for Model {
    fn aabb(&self) -> Aabb {
        match self.geometry {
            Geometry::Sphere { position, radius } => {
                Aabb::from_positions(position - radius, position + radius)
            }
            Geometry::Quad { position, u, v } => Aabb::surround(
                &Aabb::from_positions(position, position + u + v),
                &Aabb::from_positions(position + u, position + v),
            ),
            // Geometry::Quad { position, u, v } => Aabb::new(Vec3::ZERO, Vec3::MAX),
        }
    }
}

////////////////////////////////////////////////////////////////////////////////
// Scene
////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone, Encode, Decode)]
pub struct Scene {
    camera: Camera,
    objects: Vec<Arc<Model>>,
    bvh: BvhNode<Model>,
    use_bvh: bool,
}

impl Scene {
    pub fn new(camera: Camera, mut objects: Vec<Arc<Model>>) -> Self {
        let bvh = BvhNode::new(&mut objects[..]);
        Self {
            camera,
            objects,
            bvh,
            use_bvh: true,
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
        if self.use_bvh {
            self.bvh.hit(&ray)
        } else {
            let mut closest_hit_distance = ray.tmax();
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
        }
    }
}
