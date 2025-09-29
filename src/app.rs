use std::{sync::Arc, time::Duration};

use eframe::egui::{self, Color32, ColorImage, Vec2, load::Bytes};
use egui_extras::{Column, TableBuilder};
use tokio::{runtime::Runtime, task::JoinHandle};
use tracing::{debug, info, trace};

use crate::{
    peer::PeerTable,
    renderer::{self, Renderer},
};

pub struct MirrorApp {
    // Backend data
    runtime: Runtime,
    renderer: Renderer,
    // Ui data
    texture: Option<egui::TextureHandle>,
    render_future: Option<JoinHandle<()>>,
}

impl MirrorApp {
    pub fn new(runtime: Runtime, renderer: Renderer) -> Self {
        Self {
            // Backend data
            runtime,
            renderer,
            // Ui data
            texture: None,
            render_future: None,
        }
    }
}

impl eframe::App for MirrorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Force a repaint every second
        ctx.request_repaint_after(Duration::from_secs(1));

        // Build ui
        egui::CentralPanel::default().show(ctx, |ui| {
            // Render image to background
            let texture: &egui::TextureHandle = self.texture.get_or_insert_with(|| {
                // CREATING IMAGE //
                const IMAGE_SIZE: [usize; 2] = [400, 300];
                let image_bytes = Bytes::Shared(Arc::new([255u8; 400 * 300 * 3]));
                let image_data = ColorImage::from_rgb(IMAGE_SIZE, image_bytes.as_ref());

                ui.ctx()
                    .load_texture("render_image", image_data, Default::default())
            });
            egui::Image::new((texture.id(), texture.size_vec2()))
                .paint_at(ui, ui.ctx().screen_rect());

            ui.heading("Mirror Network");

            if let Ok(peer_table) = self.renderer.peer_table.try_lock() {
                if peer_table.keys().len() == 0 {
                    ui.label("No connected peers.");
                } else {
                    TableBuilder::new(ui)
                        .striped(true)
                        .resizable(false)
                        .cell_layout(egui::Layout::left_to_right(egui::Align::Min))
                        .columns(Column::remainder(), 2)
                        .body(|mut body| {
                            for (i, address) in peer_table.keys().enumerate() {
                                body.row(20.0, |mut row| {
                                    row.col(|ui| {
                                        ui.label(format!("peer{i}"));
                                    });
                                    row.col(|ui| {
                                        ui.label(address.to_string());
                                    });
                                });
                            }
                        });

                    if ui.button("Render").clicked() {
                        self.render_future = Some(self.runtime.spawn(renderer::render()));
                    }

                    if self
                        .render_future
                        .as_ref()
                        .is_some_and(|fut| fut.is_finished())
                    {
                        ui.label("Finished rendering");
                    }
                }
            } else {
                ui.label("Loading...");
            }
        });
    }
}
