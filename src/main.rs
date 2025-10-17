use std::collections::HashMap;
use std::net::SocketAddr;
use std::num::NonZero;
use std::sync::Arc;
use std::thread;

use chrono::Local;
use clap::Parser;
use glam::Vec3;
use mirror::editor;
use tokio::sync::RwLock;
use tracing::{info, warn};
use tracing_subscriber::fmt::{format::Writer, time::FormatTime};

use crate::test_scenes::*;
use mirror::config::Config;
use mirror::protocol::{Peer, listen_task};
use mirror::raytracer::Renderer;

mod test_scenes;

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
    // let runtime = tokio::runtime::Runtime::new()?;
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(
            thread::available_parallelism()
                .map(NonZero::get)
                .unwrap_or(4),
        )
        .enable_io()
        .enable_time()
        .build()
        .unwrap();

    // Load command line arguments.
    let args = Args::try_parse()?;

    // Initialize logger.
    tracing_subscriber::fmt()
        .with_env_filter("mirror=trace")
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

    let peer_table = Arc::new(RwLock::new(HashMap::<SocketAddr, Peer>::new()));
    let renderer = Arc::new(Renderer::new(peer_table.clone()));
    let scene = Arc::new(spheres2_scene(16.0 / 9.0));

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
            Box::new(|_| {
                Ok(Box::new(editor::MirrorApp::new(
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
