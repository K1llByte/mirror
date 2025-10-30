#[cfg(target_arch = "wasm32")]
use futures::{FutureExt, future::RemoteHandle};
use glam::Vec3;
use rand::Rng;
use std::num::NonZero;
#[cfg(not(target_arch = "wasm32"))]
use tokio::task::JoinHandle;

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
        radius * polar.sin() * azimuth.cos(),
        radius * polar.sin() * azimuth.sin(),
        radius * polar.cos(),
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

pub fn ideal_processors() -> usize {
    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::thread;
        thread::available_parallelism()
            .map(NonZero::get)
            .unwrap_or(1)
    }
    #[cfg(target_arch = "wasm32")]
    {
        1
    }
}

pub fn instant_now() -> f64 {
    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::time::Instant;
        static START: once_cell::sync::Lazy<Instant> = once_cell::sync::Lazy::new(Instant::now);
        START.elapsed().as_secs_f64() * 1000.0
    }
    #[cfg(target_arch = "wasm32")]
    {
        web_sys::window().unwrap().performance().unwrap().now()
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn spawn<F>(future: F) -> JoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    tokio::spawn(future)
}

#[cfg(target_arch = "wasm32")]
pub fn spawn<F>(future: F) -> RemoteHandle<()>
where
    F: Future + FutureExt + Send + 'static,
    F::Output: Send + 'static,
{
    let (fut, handle): (_, RemoteHandle<_>) = async {
        future.await;
    }
    .remote_handle();
    wasm_bindgen_futures::spawn_local(fut);
    handle
}
