use std::{sync::Arc, time::Duration};

use eframe::egui::{self, ColorImage, Key, RichText, TextureHandle, load::Bytes};
use egui_extras::{Column, TableBuilder};
use futures::FutureExt;
use tokio::{runtime::Runtime, task::JoinHandle};

use crate::renderer::{self, Renderer};

pub struct MirrorApp {
    // Backend data
    runtime: Runtime,
    renderer: Arc<Renderer>,

    // Ui data
    enable_side_panel: bool,
    texture: Option<egui::TextureHandle>,
    render_future: Option<JoinHandle<Vec<u8>>>,
}

impl MirrorApp {
    pub fn new(runtime: Runtime, renderer: Arc<Renderer>) -> Self {
        Self {
            // Backend data
            runtime,
            renderer,
            // Ui data
            enable_side_panel: true,
            texture: None,
            render_future: None,
        }
    }

    fn show_render_image(&mut self, ui: &mut egui::Ui) {
        const IMAGE_SIZE: [usize; 2] = [400, 300];

        let texture: &TextureHandle = if self
            .render_future
            .as_ref()
            .is_some_and(|fut| fut.is_finished())
        {
            let image_bytes = Bytes::Shared(Arc::from(
                self.render_future
                    .as_mut()
                    .unwrap()
                    .now_or_never()
                    .unwrap()
                    .unwrap(),
            ));
            let image_data = ColorImage::from_rgb(IMAGE_SIZE, image_bytes.as_ref());
            self.texture.replace(ui.ctx().load_texture(
                "render_image",
                image_data,
                Default::default(),
            ));
            self.render_future = None;
            self.texture.as_ref().unwrap()
        } else {
            // Create all white image
            self.texture.get_or_insert_with(|| {
                let image_bytes = Bytes::Shared(Arc::new([255u8; 400 * 300 * 3]));
                let image_data = ColorImage::from_rgb(IMAGE_SIZE, image_bytes.as_ref());

                ui.ctx()
                    .load_texture("render_image", image_data, Default::default())
            })
        };

        egui::Image::new((texture.id(), texture.size_vec2())).paint_at(ui, ui.ctx().screen_rect());
    }
}

impl eframe::App for MirrorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Force a repaint every second
        ctx.request_repaint_after(Duration::from_secs_f32(0.1));

        if ctx.input(|i| i.key_pressed(Key::Space)) {
            self.enable_side_panel = !self.enable_side_panel;
        }

        // Build ui
        if self.enable_side_panel {
            egui::SidePanel::left("side_panel").show(ctx, |ui| {
                ui.heading("Mirror");
                ui.separator();

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
                    }
                } else {
                    ui.label("Loading...");
                }

                ui.separator();

                let render_button =
                    ui.add_sized([ui.available_width(), 0.0], egui::Button::new("Render"));
                if render_button.clicked() {
                    self.render_future = Some(
                        self.runtime
                            .spawn(renderer::render_task(self.renderer.clone())),
                    );
                }
            });
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            // Render image to background
            self.show_render_image(ui);
        });
    }
}
