use core::f32;
use std::sync::Arc;

use glam::Vec3;
use rand::Rng;

use mirror::raytracer::{Camera, Material, Model, Scene, Sphere};

pub fn spheres_scene(cam_aspect_ratio: f32) -> Scene {
    // Spheres
    let sphere_left = Sphere {
        position: Vec3::new(-1.0, 0.0, -1.0),
        radius: 0.5,
    };
    let sphere_center = Sphere {
        position: Vec3::new(0.0, 0.0, -1.0),
        radius: 0.5,
    };
    let sphere_right = Sphere {
        position: Vec3::new(1.0, 0.0, -1.0),
        radius: 0.5,
    };
    let sphere_ground = Sphere {
        position: Vec3::new(0.0, -1000.5, -1.0),
        radius: 1000.0,
    };

    // Materials
    let ground_mat = Arc::new(Material::Diffuse {
        albedo: Vec3::new(0.05, 0.05, 0.05),
    });
    let center_mat = Arc::new(Material::Diffuse {
        albedo: Vec3::new(0.1, 0.2, 0.5),
    });
    let left_mat = Arc::new(Material::Dielectric {
        refraction_index: 1.5,
    });
    let right_mat = Arc::new(Material::Metalic {
        albedo: Vec3::new(0.8, 0.6, 0.2),
        fuzzyness: 0.0,
    });

    // Scene
    Scene {
        camera: Camera::new(
            Vec3::new(0.0, 6.0, 5.0),
            Vec3::new(0.0, -1.0, -1.0).normalize(),
            Vec3::new(0.0, -1.0, 0.0).normalize(),
            100.0,
            cam_aspect_ratio,
        ),
        objects: vec![
            Model {
                geometry: sphere_left,
                material: left_mat.clone(), // center_mat.clone(),
            },
            Model {
                geometry: sphere_center,
                material: center_mat.clone(),
            },
            Model {
                geometry: sphere_right,
                material: right_mat.clone(),
            },
            Model {
                geometry: sphere_ground,
                material: ground_mat.clone(),
            },
        ],
    }
}

pub fn spheres2_scene(cam_aspect_ratio: f32) -> Scene {
    let mut objects = Vec::new();

    objects.push(Model {
        geometry: Sphere {
            position: Vec3::new(0.0, -1000.5, -1.0),
            radius: 1000.0,
        },
        material: Arc::new(Material::Diffuse {
            albedo: Vec3::new(0.42, 0.42, 0.6),
        }),
    });

    let mut random_circle = |radius: f32, count: usize, mat: Arc<Material>| {
        for i in 0..count {
            let ang = (i as f32) * f32::consts::PI * 2.0 / (count as f32);

            let x = radius * f32::sin(ang);
            let z = radius * f32::cos(ang);
            objects.push(Model {
                geometry: Sphere {
                    position: Vec3 { x, y: 0.0, z },
                    radius: 0.5,
                },
                material: mat.clone(),
            });
        }
    };

    let random_diffuse = || {
        let mut rng = rand::rng();
        Arc::new(Material::Diffuse {
            albedo: Vec3::new(
                rng.random_range(0f32..=1f32),
                rng.random_range(0f32..=1f32),
                rng.random_range(0f32..=1f32),
            ),
        })
    };
    let random_dialetric = || {
        let mut rng = rand::rng();
        Arc::new(Material::Dielectric {
            refraction_index: 1.5,
        })
    };
    let random_metalic = || {
        let mut rng = rand::rng();
        Arc::new(Material::Metalic {
            albedo: Vec3::new(
                rng.random_range(0f32..=1f32),
                rng.random_range(0f32..=1f32),
                rng.random_range(0f32..=1f32),
            ),
            fuzzyness: rng.random_range(0f32..=1f32),
        })
    };
    let random_mat = || {
        let mut rng = rand::rng();
        match rng.random_range(0..3) {
            0 => random_diffuse(),
            1 => random_metalic(),
            2 => random_dialetric(),
            _ => unreachable!(),
        }
    };

    random_circle(2.0, 4, random_dialetric());
    random_circle(4.0, 8, random_metalic());
    random_circle(6.0, 16, random_diffuse());
    random_circle(8.0, 20, random_metalic());
    random_circle(10.0, 26, random_diffuse());
    random_circle(12.0, 32, random_metalic());

    Scene {
        camera: Camera::new(
            Vec3::new(0.0, 6.0, 10.0),
            Vec3::new(0.0, -1.0, -1.0).normalize(),
            Vec3::new(0.0, -1.0, 0.0).normalize(),
            100.0,
            cam_aspect_ratio,
        ),
        objects,
    }
}
