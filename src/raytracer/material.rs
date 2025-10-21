use bincode::{Decode, Encode};
use glam::Vec3;
use rand::Rng;

use crate::raytracer::{Hit, Ray};
use crate::utils;

#[derive(Debug, Clone, Encode, Decode)]
pub enum Material {
    DiffuseLight {
        #[bincode(with_serde)]
        emission: Vec3,
    },
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
            Self::DiffuseLight { .. } => None,
            Self::Diffuse { albedo } => {
                let rnd_dir = utils::random_vector(&mut rng);
                let mut direction = (hit.normal + rnd_dir).normalize();

                if direction.is_nan() {
                    direction = hit.normal;
                }

                Some(ScatteredRay {
                    ray: Ray::new(hit.position, direction),
                    attenuation: *albedo,
                })
            }
            Self::Metalic { albedo, fuzzyness } => {
                let reflected_dir = ray.direction().reflect(hit.normal).normalize();
                let mut scattered_dir =
                    (reflected_dir + *fuzzyness * utils::random_vector(&mut rng)).normalize();
                if scattered_dir.is_nan() {
                    println!("Its the metalic!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!");
                    scattered_dir = reflected_dir;
                }

                let scattered_ray = Ray::new(hit.position, scattered_dir);
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
                    let ray_direction = unit_ray_dir.reflect(hit.normal);
                    if ray_direction.normalize().is_nan() {
                        println!(
                            "Its the dialetric reflection!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!"
                        );
                    }
                    ray_direction
                } else {
                    let ray_direction = unit_ray_dir.refract(hit.normal, real_refraction_index);
                    if ray_direction.normalize().is_nan() {
                        println!(
                            "Its the dialetric refraction: unit_ray_dir={}, hit.normal={}, ray.direction={} !!",
                            unit_ray_dir,
                            hit.normal,
                            ray.direction()
                        );
                    }
                    ray_direction
                };

                Some(ScatteredRay {
                    ray: Ray::new(hit.position, ray_direction.normalize()),
                    attenuation,
                })
            }
        }
    }

    pub fn emission(&self) -> Vec3 {
        if let Self::DiffuseLight { emission } = &self {
            return *emission;
        }
        Vec3::new(0.0, 0.0, 0.0)
    }
}
