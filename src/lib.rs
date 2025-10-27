#[cfg(not(target_arch = "wasm32"))]
pub mod config;
#[cfg(not(target_arch = "wasm32"))]
pub mod editor;
#[cfg(not(target_arch = "wasm32"))]
pub mod protocol;
#[cfg(not(target_arch = "wasm32"))]
pub mod raytracer;
#[cfg(not(target_arch = "wasm32"))]
pub mod utils;

#[cfg(not(target_arch = "wasm32"))]
pub mod test_scenes;
