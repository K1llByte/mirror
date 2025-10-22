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
    Cuboid {
        #[bincode(with_serde)]
        position: Vec3,
        #[bincode(with_serde)]
        size: Vec3,
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

    fn hit_sphere(&self, ray: &Ray, position: Vec3, radius: f32) -> Option<Hit> {
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

    fn hit_quad(&self, ray: &Ray, position: Vec3, u: Vec3, v: Vec3) -> Option<Hit> {
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

    fn hit_cuboid(&self, ray: &Ray, position: Vec3, size: Vec3) -> Option<Hit> {
        let half_size = size / 2.0;
        let mut closest_hit_distance = ray.tmax();
        let mut closest_hit = None;

        let pos_x_hit = self.hit_quad(
            &ray,
            position - half_size,
            Vec3::new(0.0, 0.0, size.z),
            Vec3::new(0.0, size.y, 0.0),
        );
        let neg_x_hit = self.hit_quad(
            &ray,
            position + half_size,
            Vec3::new(0.0, -size.y, 0.0),
            Vec3::new(0.0, 0.0, -size.z),
        );
        let neg_z_hit = self.hit_quad(
            &ray,
            position - half_size,
            Vec3::new(0.0, size.y, 0.0),
            Vec3::new(size.x, 0.0, 0.0),
        );
        let pos_z_hit = self.hit_quad(
            &ray,
            position + half_size,
            Vec3::new(-size.x, 0.0, 0.0),
            Vec3::new(0.0, -size.y, 0.0),
        );
        let neg_y_hit = self.hit_quad(
            &ray,
            position - half_size,
            Vec3::new(size.x, 0.0, 0.0),
            Vec3::new(0.0, 0.0, size.z),
        );
        let pos_y_hit = self.hit_quad(
            &ray,
            position + half_size,
            Vec3::new(0.0, 0.0, -size.z),
            Vec3::new(-size.x, 0.0, 0.0),
        );
        let quads_hits = [
            pos_x_hit, neg_x_hit, pos_z_hit, neg_z_hit, pos_y_hit, neg_y_hit,
        ];
        for quad_hit in quads_hits {
            if let Some(hit) = quad_hit {
                if hit.distance < closest_hit_distance {
                    closest_hit_distance = hit.distance;
                    closest_hit = Some(hit);
                }
            }
        }

        closest_hit
    }
}

impl Hittable for Model {
    fn hit(&self, ray: &Ray) -> Option<Hit> {
        match self.geometry {
            Geometry::Sphere { position, radius } => self.hit_sphere(&ray, position, radius),
            Geometry::Quad { position, u, v } => self.hit_quad(&ray, position, u, v),
            Geometry::Cuboid { position, size } => self.hit_cuboid(&ray, position, size),
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
            Geometry::Cuboid { position, size } => Aabb::new(position, size),
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
    #[bincode(with_serde)]
    background: Vec3,
    bvh: BvhNode<Model>,
    use_bvh: bool,
}

impl Scene {
    pub fn new(camera: Camera, mut objects: Vec<Arc<Model>>) -> Self {
        let bvh = BvhNode::new(&mut objects[..]);
        Self {
            camera,
            objects,
            background: Vec3::ZERO,
            bvh,
            use_bvh: true,
        }
    }

    pub fn with_background(camera: Camera, mut objects: Vec<Arc<Model>>, background: Vec3) -> Self {
        let bvh = BvhNode::new(&mut objects[..]);
        Self {
            camera,
            objects,
            background,
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

    pub fn background(&self) -> Vec3 {
        self.background
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
