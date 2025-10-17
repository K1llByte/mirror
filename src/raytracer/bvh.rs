use core::f32;
use std::sync::Arc;

use bincode::{Decode, Encode};
use tracing::debug;

use crate::raytracer::{Aabb, Hit, Hittable, Intersectable, Model, Ray};

pub trait Bounded {
    fn aabb(&self) -> Aabb;
}

impl Bounded for Model {
    fn aabb(&self) -> Aabb {
        Aabb::from_positions(
            self.geometry.position - self.geometry.radius,
            self.geometry.position + self.geometry.radius,
        )
    }
}

#[derive(Debug, Clone, Encode, Decode)]
pub enum BvhNode<H: Hittable + Bounded> {
    Branch {
        left: Arc<BvhNode<H>>,
        right: Arc<BvhNode<H>>,
        aabb: Aabb,
    },
    Leaf(Arc<H>),
}

impl<H: Hittable + Bounded> BvhNode<H> {
    pub fn new(elems: &mut [Arc<H>]) -> Self {
        assert!(elems.len() > 0, "Cannot create a BVH with 0 elements");

        let mut aabb = Aabb::empty();
        for h in elems.iter() {
            aabb = Aabb::surround(&aabb, &h.aabb());
        }
        let cmp_axis = (aabb.max_position - aabb.min_position).max_position();

        match elems.len() {
            1 => Self::Leaf(elems[0].clone()),
            _ => {
                elems.sort_by(|a, b| {
                    a.aabb().min_position[cmp_axis].total_cmp(&b.aabb().min_position[cmp_axis])
                });
                let mid = elems.len() / 2;
                let (left_slice, right_slice) = elems.split_at_mut(mid);
                let left = Arc::new(BvhNode::new(left_slice));
                let right = Arc::new(BvhNode::new(right_slice));

                Self::Branch { left, right, aabb }
            }
        }
    }

    pub fn aabb(&self) -> Aabb {
        match self {
            Self::Branch { aabb, .. } => aabb.clone(),
            Self::Leaf(obj) => obj.aabb(),
        }
    }

    pub fn depth(&self) -> usize {
        match self {
            Self::Branch { left, right, .. } => left.depth().max(right.depth()) + 1,
            Self::Leaf(_) => 1,
        }
    }
}

impl<H: Hittable + Bounded> Hittable for BvhNode<H> {
    fn hit(&self, ray: &Ray) -> Option<Hit> {
        match self {
            Self::Branch { left, right, aabb } => {
                if !aabb.intersect(&ray) {
                    return None;
                }

                let left_hit = left.hit(&ray);
                let right_hit = if let Some(h) = &left_hit {
                    right.hit(&ray.with_tmax(h.distance))
                } else {
                    right.hit(&ray)
                };

                if right_hit.is_some() {
                    right_hit
                } else {
                    left_hit
                }
            }
            Self::Leaf(obj) => {
                if !obj.aabb().intersect(&ray) {
                    return None;
                }
                obj.hit(&ray)
            }
        }
    }
}
