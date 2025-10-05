use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use chrono::Local;
use clap::Parser;
use glam::Vec3;
use tokio::sync::Mutex;
use tracing::info;
use tracing_subscriber::fmt::{format::Writer, time::FormatTime};

use crate::camera::Camera;
use crate::config::Config;
use crate::peer::{Peer, listen_task};
use crate::renderer::Renderer;
use crate::scene::{Scene, Sphere};

mod app;
mod camera;
mod config;
mod image;
mod packet;
mod peer;
mod ray;
mod renderer;
mod scene;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to config toml file
    #[arg(short, long)]
    config: Option<String>,
    #[arg(short, long, default_value_t = false)]
    no_gui: bool,
}

struct CustomTime;

impl FormatTime for CustomTime {
    fn format_time(&self, w: &mut Writer<'_>) -> std::fmt::Result {
        write!(w, "[{}]", Local::now().format("%H:%M:%S"))
        // write!(w, "[{}]", Local::now().format("%d/%m/%y %H:%M:%S"))
    }
}

fn main() -> anyhow::Result<()> {
    // Create tokio runtime.
    let runtime = tokio::runtime::Runtime::new()?;
    // Load command line arguments.
    let args = Args::try_parse()?;

    // Initialize logger.
    tracing_subscriber::fmt()
        .with_env_filter("mirror=debug")
        .with_timer(CustomTime)
        .init();

    // Load config file if specified, otherwise use default.
    let config = match &args.config {
        Some(path) => {
            info!("Loaded config from '{}'", path);
            runtime.block_on(Config::from_file(&path))?
        }
        None => {
            info!("Using default config");
            Default::default()
        }
    };

    let peer_table = Arc::new(Mutex::new(HashMap::<SocketAddr, Peer>::new()));
    let renderer = Arc::new(Renderer::new(peer_table.clone()));
    let scene = {
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
            position: Vec3::new(0.0, -100.5, -1.0),
            radius: 100.0,
        };
        Arc::new(Scene {
            camera: Camera::new(Vec3::ZERO, 1920f32, 1080f32),
            objects: vec![sphere_left, sphere_center, sphere_right, sphere_ground],
        })
    };

    let listen_task_future = runtime.spawn(listen_task(
        renderer.clone(),
        config.host,
        config.bootstrap_peers,
    ));

    if !args.no_gui {
        let options = eframe::NativeOptions::default();
        eframe::run_native(
            "Mirror App",
            options,
            Box::new(|_cc| {
                Ok(Box::new(app::MirrorApp::new(
                    runtime,
                    renderer.clone(),
                    scene,
                )))
            }),
        )
        .unwrap();
    } else {
        runtime.block_on(listen_task_future)??;
    }

    Ok(())
}
