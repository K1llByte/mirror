use glam::Vec3;
use rand::Rng;

/// Reflect vector
pub fn reflect(v: Vec3, n: Vec3) -> Vec3 {
    v - 2.0 * v.dot(n) * n
}

/// Refract vector
pub fn refract(v: Vec3, n: Vec3, factor: f32) -> Vec3 {
    let cos_theta = (-v).dot(n).min(1.0);
    let r_out_perp = factor * (v + cos_theta * n);
    let r_out_parallel = (-((1.0 - r_out_perp.length_squared()).abs().sqrt())) * n;
    r_out_perp + r_out_parallel
}

/// Convert cartesian into spherical coordinates
pub fn cartesian_to_spherical(v: Vec3) -> Vec3 {
    let radius = (v.x * v.x + v.y * v.y + v.z * v.z).sqrt();
    let polar = v.x.atan2(v.z);
    let azimuth = (v.y / radius).acos();
    Vec3::new(radius, polar, azimuth)
}

/// Convert spherical into cartesian coordinates
pub fn spherical_to_cartesian(v: Vec3) -> Vec3 {
    // radius [0,+inf[
    let radius = v.x;
    // polar [0,pi]
    let polar = v.y;
    // azimuth [0,2pi[
    let azimuth = v.z;

    Vec3::new(
        radius * polar.sin() * azimuth.sin(),
        radius * azimuth.cos(),
        radius * azimuth.sin() * polar.cos(),
    )
}

/// Return a normalized random vector
pub fn random_vector(rng: &mut impl Rng) -> Vec3 {
    let polar = rng.random_range(0.0..std::f32::consts::PI);
    let azimuth = rng.random_range(0.0..(2.0 * std::f32::consts::PI));

    spherical_to_cartesian(Vec3::new(1.0, polar, azimuth))
}

/// Return a normalized random vector in the hemisphere of a normal
pub fn random_in_hemisphere(rng: &mut impl Rng, normal: Vec3) -> Vec3 {
    let vec = random_vector(rng);
    if vec.dot(normal) > 0.0 {
        return vec;
    } else {
        return -vec;
    }
}
