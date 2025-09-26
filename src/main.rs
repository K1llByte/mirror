use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use clap::Parser;
use tokio::sync::Mutex;
use tracing::info;

use crate::config::Config;
use crate::peer::{Peer, listen_task};

mod app;
mod config;
mod packet;
mod peer;
mod scene;
// mod renderer;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to config toml file
    #[arg(short, long)]
    config: Option<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load command line arguments.
    let args = Args::try_parse()?;

    // Initialize logger.
    tracing_subscriber::fmt()
        .with_env_filter("mirror=trace")
        .init();

    // Load config file if specified, otherwise use default.
    let config = match &args.config {
        Some(path) => {
            info!("Loaded config from '{}'", path);
            Config::from_file(&path).await?
        }
        None => {
            info!("Using default config");
            Default::default()
        }
    };

    let peer_table = Arc::new(Mutex::new(HashMap::<SocketAddr, Peer>::new()));

    tokio::spawn(listen_task(
        peer_table.clone(),
        config.host,
        config.bootstrap_peers,
    ));

    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Mirror App",
        options,
        Box::new(|_cc| Ok(Box::new(app::MirrorApp::new(peer_table)))),
    )
    .unwrap();

    Ok(())
}
