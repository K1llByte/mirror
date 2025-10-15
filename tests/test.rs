use glam::Vec3;
use mirror::raytracer::{Aabb, Intersectable, Ray};

#[test]
fn aabb_inner_intersection() {
    let aabb = Aabb::new(Vec3::ZERO, Vec3::new(1.0, 1.0, 1.0));
    let ray = Ray::new(Vec3::ZERO, Vec3::new(1.0, 0.0, 0.0));
    assert_eq!(aabb.intersect(&ray), true);
}

#[test]
fn aabb_corner_intersection() {
    let aabb = Aabb::new(Vec3::ZERO, Vec3::new(1.0, 1.0, 1.0));
    let ray = Ray::new(
        Vec3::new(-1.0, 0.0, 0.0),
        Vec3::new(1.0, -1.0, 0.0).normalize(),
    );
    assert_eq!(aabb.intersect(&ray), true);
}

#[test]
fn aabb_no_intersection() {
    let aabb = Aabb::new(Vec3::ZERO, Vec3::new(1.0, 1.0, 1.0));
    let ray = Ray::new(
        Vec3::new(-2.0, 0.0, 0.0),
        Vec3::new(1.0, -1.0, 0.0).normalize(),
    );
    assert_eq!(aabb.intersect(&ray), false);
}

#[test]
fn aabb_inverse_intersection() {
    let aabb = Aabb::new(Vec3::ZERO, Vec3::new(1.0, 1.0, 1.0));
    let ray = Ray::new(Vec3::new(-1.0, 0.0, 0.0), Vec3::new(-1.0, 0.0, 0.0));
    assert_eq!(aabb.intersect(&ray), false);
}

#[test]
fn aabb_tangent_intersection() {
    let aabb = Aabb::new(Vec3::ZERO, Vec3::new(1.0, 1.0, 1.0));
    let ray = Ray::new(Vec3::new(-1.0, 0.0, 0.0), Vec3::new(0.0, -1.0, 0.0));
    assert_eq!(aabb.intersect(&ray), false);
}
