use std::cmp::min;

use bincode::{Decode, Encode};
use glam::Vec3;
use rand::Rng;

use crate::{ray::Ray, scene::Hit, utils};

#[derive(Debug, Clone, Encode, Decode)]
pub enum Material {
    Diffuse {
        #[bincode(with_serde)]
        albedo: Vec3,
    },
    Metalic {
        #[bincode(with_serde)]
        albedo: Vec3,
        fuzzyness: f32,
    },
    Dielectric {
        refraction_index: f32,
    },
}

pub struct ScatteredRay {
    pub ray: Ray,
    pub attenuation: Vec3,
}

impl Material {
    pub fn scatter(&self, ray: &Ray, hit: &Hit) -> Option<ScatteredRay> {
        let mut rng = rand::rng();

        match self {
            Self::Diffuse { albedo } => {
                let direction = hit.normal + utils::random_vector(&mut rng);

                // TODO: Check if direction is not near 0

                Some(ScatteredRay {
                    ray: Ray::new(hit.position, direction),
                    attenuation: *albedo,
                })
            }
            Self::Metalic { albedo, fuzzyness } => {
                let reflected_dir = utils::reflect(ray.direction(), hit.normal).normalize();
                let scattered_ray = Ray::new(
                    hit.position,
                    reflected_dir + *fuzzyness * utils::random_vector(&mut rng),
                );
                if scattered_ray.direction().dot(hit.normal) > 0.0 {
                    Some(ScatteredRay {
                        ray: scattered_ray,
                        attenuation: *albedo,
                    })
                } else {
                    None
                }
            }
            Self::Dielectric { refraction_index } => {
                let attenuation = Vec3::new(1.0, 1.0, 1.0);
                let real_refraction_index = if hit.is_front_face {
                    1.0 / *refraction_index
                } else {
                    *refraction_index
                };

                let unit_ray_dir = ray.direction().normalize();
                let cos_theta = f32::min((-unit_ray_dir).dot(hit.normal), 1.0);
                let sin_theta = f32::sqrt(1.0 - cos_theta * cos_theta);
                let cannot_refract = real_refraction_index * sin_theta > 1.0;

                let schlick_approximation = |cosine: f32, ri: f32| {
                    let r0 = (1.0 - ri) / (1.0 + ri);
                    let r0_squared = r0 * r0;
                    r0_squared + (1.0 - r0_squared) * f32::powf(1.0 - cosine, 5.0)
                };

                let ray_direction = if cannot_refract
                    || schlick_approximation(cos_theta, real_refraction_index)
                        > rng.random_range(0f32..1f32)
                {
                    utils::reflect(unit_ray_dir, hit.normal)
                } else {
                    utils::refract(unit_ray_dir, hit.normal, real_refraction_index)
                };

                Some(ScatteredRay {
                    ray: Ray::new(hit.position, ray_direction),
                    attenuation,
                })
            }
        }
    }
}
