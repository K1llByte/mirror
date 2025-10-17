use std::rc::Rc;

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

pub enum BvhNode<'h, H: Hittable + Bounded> {
    Branch {
        left: Rc<BvhNode<'h, H>>,
        right: Rc<BvhNode<'h, H>>,
        aabb: Aabb,
    },
    Leaf(&'h [H]),
}

// impl<'h, H: Hittable + Bounded> BvhNode<'h, H> {
//     pub fn new(elems: &'h mut [H]) -> Self {
//         let axis = 0;

//         match elems.len() {
//             0 => unreachable!(),
//             1 => Self::Leaf(&elems[0..]),
//             // 2 => (&mut elems[0..1], &mut elems[1..2]),
//             _ => {
//                 // todo!();
//                 elems.sort_by(|a, b| {
//                     // a.aabb()
//                     // b.aabb()
//                     todo!();
//                 });
//                 let mid = elems.len() / 2;
//                 let left = Rc::new(BvhNode::new(&mut elems[0..mid]));
//                 let right = Rc::new(BvhNode::new(&mut elems[mid..]));
//                 let aabb = Aabb::surrounding_box(&left.aabb(), &right.aabb());

//                 Self::Branch { left, right, aabb }
//             }
//         }
//     }

//     pub fn aabb(&self) -> Aabb {
//         match self {
//             Self::Branch { aabb, .. } => aabb.clone(),
//             Self::Leaf(obj) => obj[0].aabb(),
//         }
//     }
// }

impl<'h, H: Hittable + Bounded> Hittable for BvhNode<'h, H> {
    fn hit(&self, ray: &Ray) -> Option<Hit> {
        match self {
            Self::Branch { left, right, aabb } => {
                if !aabb.intersect(&ray) {
                    return None;
                }

                let left_hit = left.hit(&ray);
                let right_hit = right.hit(&ray);

                // FIXME: This depends on tmin/tmax rafactor
                if right_hit.is_some() {
                    right_hit
                } else {
                    left_hit
                }
            }
            Self::Leaf(obj) => {
                if obj[0].aabb().intersect(&ray) {
                    return None;
                }
                obj[0].hit(&ray)
            }
        }
    }
}
