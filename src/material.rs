use bincode::{Decode, Encode};
use glam::Vec3;

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

                let ri = if hit.is_front_face {
                    1.0 / *refraction_index
                } else {
                    *refraction_index
                };

                let refracted = utils::refract(ray.direction().normalize(), hit.normal, ri);

                Some(ScatteredRay {
                    ray: Ray::new(hit.position, refracted),
                    attenuation,
                })
            }
        }
    }
}
