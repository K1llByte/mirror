use core::f32;
use std::{sync::Arc, time::Instant};

use glam::Vec3;
use rand::Rng;

use mirror::raytracer::{BvhNode, Camera, Geometry, Material, Model, Scene};
use tracing::debug;

pub fn spheres_scene(cam_aspect_ratio: f32) -> Scene {
    // Spheres
    let sphere_left = Geometry::Sphere {
        position: Vec3::new(-1.0, 0.0, -1.0),
        radius: 0.5,
    };
    let sphere_center = Geometry::Sphere {
        position: Vec3::new(0.0, 0.0, -1.0),
        radius: 0.5,
    };
    let sphere_right = Geometry::Sphere {
        position: Vec3::new(1.0, 0.0, -1.0),
        radius: 0.5,
    };
    let sphere_ground = Geometry::Sphere {
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
    Scene::with_background(
        Camera::new(
            Vec3::new(0.0, 6.0, 5.0),
            Vec3::new(0.0, -1.0, -1.0).normalize(),
            Vec3::new(0.0, -1.0, 0.0).normalize(),
            100.0,
            cam_aspect_ratio,
        ),
        vec![
            Arc::new(Model {
                geometry: sphere_left,
                material: left_mat.clone(),
            }),
            Arc::new(Model {
                geometry: sphere_center,
                material: center_mat.clone(),
            }),
            Arc::new(Model {
                geometry: sphere_right,
                material: right_mat.clone(),
            }),
            Arc::new(Model {
                geometry: sphere_ground,
                material: ground_mat.clone(),
            }),
        ],
        Vec3::new(0.70, 0.80, 1.00),
    )
}

pub fn spheres2_scene(cam_aspect_ratio: f32) -> Scene {
    let mut objects = Vec::new();

    // Ground sphere
    objects.push(Arc::new(Model {
        geometry: Geometry::Sphere {
            position: Vec3::new(0.0, -1000.5, -1.0),
            radius: 1000.0,
        },
        material: Arc::new(Material::Diffuse {
            albedo: Vec3::new(0.42, 0.42, 0.6),
        }),
    }));

    let mut random_circle = |radius: f32, count: usize, mat: Arc<Material>| {
        for i in 0..count {
            let ang = (i as f32) * f32::consts::PI * 2.0 / (count as f32);

            let x = radius * f32::sin(ang);
            let z = radius * f32::cos(ang);
            objects.push(Arc::new(Model {
                geometry: Geometry::Sphere {
                    position: Vec3 { x, y: 0.0, z },
                    radius: 0.5,
                },
                material: mat.clone(),
            }));
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
    random_circle(14.0, 50, random_metalic());
    random_circle(16.0, 60, random_metalic());

    Scene::with_background(
        Camera::new(
            Vec3::new(0.0, 1.0, 10.0),
            Vec3::new(0.0, -0.3, -1.0).normalize(),
            Vec3::new(0.0, -1.0, 0.0).normalize(),
            100.0,
            cam_aspect_ratio,
        ),
        objects,
        Vec3::new(0.70, 0.80, 1.00),
    )
}

pub fn quads_scene(cam_aspect_ratio: f32) -> Scene {
    let mut objects = Vec::new();

    let right_mat = Arc::new(Material::Diffuse {
        albedo: Vec3::new(1.0, 0.2, 0.2),
    });
    let left_mat = Arc::new(Material::Diffuse {
        albedo: Vec3::new(0.2, 1.0, 0.2),
    });
    let front_mat = Arc::new(Material::Diffuse {
        albedo: Vec3::new(0.2, 0.2, 1.0),
    });
    let up_mat = Arc::new(Material::Diffuse {
        albedo: Vec3::new(1.0, 0.5, 0.0),
    });
    let down_mat = Arc::new(Material::Diffuse {
        albedo: Vec3::new(0.2, 0.8, 0.8),
    });

    objects.push(Arc::new(Model::new(
        Geometry::Quad {
            position: Vec3::new(-3.0, -2.0, 5.0),
            u: Vec3::new(0.0, 0.0, -4.0),
            v: Vec3::new(0.0, 4.0, 0.0),
        },
        right_mat.clone(),
    )));

    objects.push(Arc::new(Model::new(
        Geometry::Quad {
            position: Vec3::new(-2.0, -2.0, 0.0),
            u: Vec3::new(4.0, 0.0, 0.0),
            v: Vec3::new(0.0, 4.0, 0.0),
        },
        front_mat.clone(),
    )));

    objects.push(Arc::new(Model::new(
        Geometry::Quad {
            position: Vec3::new(3.0, -2.0, 1.0),
            u: Vec3::new(0.0, 0.0, 4.0),
            v: Vec3::new(0.0, 4.0, 0.0),
        },
        left_mat.clone(),
    )));

    objects.push(Arc::new(Model::new(
        Geometry::Quad {
            position: Vec3::new(-2.0, 3.0, 1.0),
            u: Vec3::new(4.0, 0.0, 0.0),
            v: Vec3::new(0.0, 0.0, 4.0),
        },
        up_mat.clone(),
    )));

    objects.push(Arc::new(Model::new(
        Geometry::Quad {
            position: Vec3::new(-2.0, -3.0, 5.0),
            u: Vec3::new(4.0, 0.0, 0.0),
            v: Vec3::new(0.0, 0.0, -4.0),
        },
        down_mat.clone(),
    )));

    objects.push(Arc::new(Model::new(
        Geometry::Sphere {
            position: Vec3::new(0.0, 0.0, 2.0),
            radius: 1.0,
        },
        Arc::new(Material::DiffuseLight {
            emission: Vec3::new(4.0, 4.0, 4.0),
        }),
    )));

    Scene::new(
        Camera::new(
            Vec3::new(0.0, 0.0, 8.0),
            Vec3::new(0.0, 0.0, -1.0).normalize(),
            Vec3::new(0.0, -1.0, 0.0).normalize(),
            90.0,
            1.0, //cam_aspect_ratio,
        ),
        objects,
    )
}

pub fn cornell_scene(cam_aspect_ratio: f32) -> Scene {
    let mut objects = Vec::new();

    let red_mat = Arc::new(Material::Diffuse {
        albedo: Vec3::new(0.65, 0.05, 0.05),
    });
    let green_mat = Arc::new(Material::Diffuse {
        albedo: Vec3::new(0.12, 0.45, 0.15),
    });
    let white_mat = Arc::new(Material::Diffuse {
        albedo: Vec3::new(0.73, 0.73, 0.73),
    });
    let light_mat = Arc::new(Material::DiffuseLight {
        emission: Vec3::new(15.0, 15.0, 15.0),
    });
    let metal_mat = Arc::new(Material::Metalic {
        albedo: Vec3::new(0.8, 0.65, 0.7),
        fuzzyness: 0.2,
    });
    let glass_mat = Arc::new(Material::Dielectric {
        refraction_index: 1.5,
    });

    objects.push(Arc::new(Model::new(
        Geometry::Quad {
            position: Vec3::new(555.0, 0.0, 0.0),
            u: Vec3::new(0.0, 0.0, 555.0),
            v: Vec3::new(0.0, 555.0, 0.0),
        },
        green_mat.clone(),
    )));
    objects.push(Arc::new(Model::new(
        Geometry::Quad {
            position: Vec3::new(0.0, 0.0, 0.0),
            u: Vec3::new(0.0, 555.0, 0.0),
            v: Vec3::new(0.0, 0.0, 555.0),
        },
        red_mat.clone(),
    )));
    objects.push(Arc::new(Model::new(
        Geometry::Quad {
            position: Vec3::new(0.0, 0.0, 0.0),
            u: Vec3::new(0.0, 0.0, 555.0),
            v: Vec3::new(555.0, 0.0, 0.0),
        },
        white_mat.clone(),
    )));
    objects.push(Arc::new(Model::new(
        Geometry::Quad {
            position: Vec3::new(555.0, 555.0, 555.0),
            u: Vec3::new(-555.0, 0.0, 0.0),
            v: Vec3::new(0.0, 0.0, -555.0),
        },
        white_mat.clone(),
    )));
    objects.push(Arc::new(Model::new(
        Geometry::Quad {
            position: Vec3::new(0.0, 0.0, 555.0),
            u: Vec3::new(0.0, 555.0, 0.0),
            v: Vec3::new(555.0, 0.0, 0.0),
        },
        white_mat.clone(),
    )));

    // Glass sphere
    objects.push(Arc::new(Model::new(
        Geometry::Sphere {
            position: Vec3::new(405.0, 100.0, 240.0),
            radius: 100.0,
        },
        glass_mat.clone(),
    )));

    // Metal sphere
    objects.push(Arc::new(Model::new(
        Geometry::Sphere {
            position: Vec3::new(150.0, 100.0, 360.0),
            radius: 100.0,
        },
        metal_mat.clone(),
    )));

    // Metal cuboid
    // objects.push(Arc::new(Model::new(
    //     Geometry::Cuboid {
    //         position: Vec3::new(150.0, 0.0, 360.0),
    //         size: Vec3::splat(100.0),
    //     },
    //     metal_mat.clone(),
    // )));

    // Light
    objects.push(Arc::new(Model::new(
        Geometry::Quad {
            position: Vec3::new(343.0, 554.0, 332.0),
            u: Vec3::new(-130.0, 0.0, 0.0),
            v: Vec3::new(0.0, 0.0, -105.0),
        },
        light_mat.clone(),
    )));

    Scene::with_background(
        Camera::new(
            Vec3::new(278.0, 278.0, -800.0),
            Vec3::new(0.0, 0.0, 1.0).normalize(),
            Vec3::new(0.0, -1.0, 0.0).normalize(),
            40.0,
            1.0, //cam_aspect_ratio,
        ),
        objects,
        // Vec3::new(0.70, 0.80, 1.00),
        Vec3::new(0.0, 0.0, 0.0),
    )
}
