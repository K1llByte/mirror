use std::{sync::Arc, time::Duration};

use chrono::Local;
use eframe::egui::{
    self, Color32, ColorImage, DragValue, Grid, Key, Margin, RichText, TextureHandle, Ui,
    load::Bytes,
};
use egui_extras::{Column, TableBuilder};
use futures::FutureExt;
use image::{ImageBuffer, RgbImage};
use tokio::{runtime::Runtime, sync::RwLock, task::JoinHandle};

use crate::raytracer::{self, AccumulatedImage, RenderBackend, RenderInfo, Scene};

pub struct MirrorApp {
    // Backend data
    runtime: Runtime,
    render_backend: RenderBackend,
    render_image: Arc<RwLock<AccumulatedImage>>,
    scene: Arc<Scene>,

    // Ui data
    enable_side_panel: bool,
    // Background
    present_framebuffer: bool,
    texture: Option<egui::TextureHandle>,
    render_join_handle: Option<JoinHandle<RenderInfo>>,
    // Rendering
    progressive_rendering: bool,
    samples_per_pixel: usize,
    framebuffer_size: (usize, usize),
    // Network
    cached_peers_info: Vec<(Option<String>, String)>,
    // Render info
    render_info: RenderInfo,
}

impl MirrorApp {
    pub fn new(runtime: Runtime, render_backend: RenderBackend, scene: Arc<Scene>) -> Self {
        let framebuffer_size = (400, 400);
        Self {
            // Backend data
            runtime,
            render_backend,
            render_image: Arc::new(RwLock::new(AccumulatedImage::new(framebuffer_size))),
            scene,
            // Ui data
            present_framebuffer: false,
            enable_side_panel: true,
            texture: None,
            render_join_handle: None,
            progressive_rendering: false,
            samples_per_pixel: 20,
            framebuffer_size,
            cached_peers_info: vec![],
            render_info: RenderInfo::default(),
        }
    }

    fn spawn_render_task(&mut self) {
        self.render_join_handle = Some(self.runtime.spawn(raytracer::render_task(
            self.render_backend.clone(),
            self.render_image.clone(),
            self.scene.clone(),
            self.samples_per_pixel,
        )));
    }

    fn show_render_image(&mut self, ui: &mut egui::Ui) {
        let has_render_finished = self.render_join_handle.as_mut().is_some_and(|jh| {
            jh.is_finished()
                .then(|| self.render_info.merge(&jh.now_or_never().unwrap().unwrap()))
                .is_some()
        });
        let texture: &TextureHandle = if has_render_finished || self.present_framebuffer {
            let (image_size, image_bytes): ([usize; 2], _) = {
                let render_image_guard = self.render_image.blocking_read();
                (
                    render_image_guard.size().into(),
                    Bytes::Shared(Arc::from(render_image_guard.to_bytes())),
                )
            };
            let image_data = ColorImage::from_rgb(image_size, image_bytes.as_ref());
            self.texture.replace(ui.ctx().load_texture(
                "render_image",
                image_data,
                Default::default(),
            ));

            if self.progressive_rendering {
                self.spawn_render_task();
            } else {
                self.render_join_handle = None;
            };
            self.present_framebuffer = false;

            self.texture.as_ref().unwrap()
        } else {
            self.texture.get_or_insert_with(|| {
                // This is only to fill the background with a default color
                let image_bytes = Bytes::Shared(Arc::from(vec![20u8; 3]));
                let image_data = ColorImage::from_rgb([1, 1], image_bytes.as_ref());

                ui.ctx()
                    .load_texture("render_image", image_data, Default::default())
            })
        };

        egui::Image::new((texture.id(), texture.size_vec2())).paint_at(ui, ui.ctx().screen_rect());
    }

    fn show_network(&mut self, ui: &mut egui::Ui) {
        ui.heading(RichText::new("Network").color(Color32::LIGHT_GRAY));

        // NOTE: Since Im using try_lock to get peers info to avoid blocking
        // ui task, I use a Vec to cache the info when its not possible to get
        // the lock guard.
        if let Ok(peer_table_guard) = self.render_backend.peer_table.try_read() {
            self.cached_peers_info = peer_table_guard
                .keys()
                .map(|a| (peer_table_guard.get(a).unwrap().name.clone(), a.to_string()))
                .collect();
        }

        if self.cached_peers_info.len() == 0 {
            ui.label("No connected peers.");
        } else {
            TableBuilder::new(ui)
                .striped(true)
                .resizable(false)
                .cell_layout(egui::Layout::left_to_right(egui::Align::Min))
                .columns(Column::remainder(), 2)
                .body(|mut body| {
                    for (i, (name, address)) in self.cached_peers_info.iter().enumerate() {
                        body.row(20.0, |mut row| {
                            row.col(|ui| {
                                ui.label(format!(
                                    "{}: {}",
                                    i + 1,
                                    name.as_deref().unwrap_or("<unnamed>")
                                ));
                            });
                            row.col(|ui| {
                                ui.label(address.to_string());
                            });
                        });
                    }
                });
        }
    }

    fn save_render_image(&self, path: &str) {
        let render_image_guard = self.render_image.blocking_read();
        let (width, height) = render_image_guard.size();

        let img: RgbImage = ImageBuffer::from_raw(
            width as u32,
            height as u32,
            render_image_guard.to_bytes().to_vec(),
        )
        .expect("Failed to create image buffer");

        img.save(path).expect("Couldn't save render image file");
    }

    fn show_rendering(&mut self, ui: &mut egui::Ui) {
        ui.heading(RichText::new("Rendering").color(Color32::LIGHT_GRAY));

        let is_rendering = self
            .render_join_handle
            .as_ref()
            .is_some_and(|fut| !fut.is_finished());
        // Render button
        let render_button = ui.add_enabled(!is_rendering, |ui: &mut Ui| {
            TableBuilder::new(ui)
                .resizable(false)
                .cell_layout(egui::Layout::left_to_right(egui::Align::Min))
                .columns(Column::remainder(), 2)
                .body(|mut body| {
                    // Framebuffer size value
                    body.row(20.0, |mut row| {
                        row.col(|ui| {
                            ui.label("Framebuffer size");
                        });
                        row.col(|ui| {
                            let fb_width_drag =
                                ui.add(DragValue::new(&mut self.framebuffer_size.0));
                            let fb_height_drag =
                                ui.add(DragValue::new(&mut self.framebuffer_size.1));
                            if fb_width_drag.changed() || fb_height_drag.changed() {
                                self.render_image
                                    .blocking_write()
                                    .resize(self.framebuffer_size);
                            }
                        });
                    });
                    // Samples per pixel value
                    body.row(20.0, |mut row| {
                        row.col(|ui| {
                            ui.label("Samples per pixel");
                        });
                        row.col(|ui| {
                            ui.add(DragValue::new(&mut self.samples_per_pixel));
                        });
                    });
                });

            // Progressive rendering checkbox
            ui.checkbox(&mut self.progressive_rendering, "Progressive Rendering");

            // Render Button
            ui.add_sized(
                [ui.available_width(), 30.0],
                egui::Button::new(if is_rendering {
                    "Rendering ..."
                } else {
                    "Render"
                }),
            )
        });
        if render_button.clicked() {
            self.spawn_render_task();
        }

        // Stop button
        let stop_button = ui.add_enabled(is_rendering, |ui: &mut Ui| {
            ui.add_sized(
                [ui.available_width(), 30.0],
                egui::Button::new("Stop").fill(Color32::from_rgb(100, 40, 40)),
            )
        });
        if stop_button.clicked() {
            self.progressive_rendering = false;
            // TODO: Explicit tasks cancelation (including all child tasks)
        }

        let save_image_button =
            ui.add_sized([ui.available_width(), 0.0], egui::Button::new("Save Image"));
        if save_image_button.clicked() {
            self.save_render_image(
                format!("render_{}.png", Local::now().format("%Y%m%d_%H%M%S")).as_str(),
            );
        }
    }

    fn show_render_info(&mut self, ui: &mut egui::Ui) {
        ui.heading(RichText::new("Render Info").color(Color32::LIGHT_GRAY));

        TableBuilder::new(ui)
            .id_salt("render_info")
            .striped(true)
            .resizable(false)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Min))
            .columns(Column::remainder(), 2)
            .body(|mut body| {
                body.row(20.0, |mut row| {
                    row.col(|ui| {
                        ui.label("Total time");
                    });
                    row.col(|ui| {
                        ui.label(format!("{} ms", self.render_info.total_time));
                    });
                });
                body.row(20.0, |mut row| {
                    row.col(|ui| {
                        ui.label("Total samples");
                    });
                    row.col(|ui| {
                        ui.label(self.render_info.total_samples.to_string());
                    });
                });
                body.row(20.0, |mut row| {
                    row.col(|ui| {
                        ui.label("Last render time");
                    });
                    row.col(|ui| {
                        ui.label(format!("{} ms", self.render_info.last_time));
                    });
                });
                body.row(20.0, |mut row| {
                    row.col(|ui| {
                        ui.label("Last render samples");
                    });
                    row.col(|ui| {
                        ui.label(self.render_info.last_samples.to_string());
                    });
                });
                body.row(20.0, |mut row| {
                    row.col(|ui| {
                        ui.label("Total avg time per sample");
                    });
                    row.col(|ui| {
                        ui.label(format!("{} ms", self.render_info.total_avg_time_per_sample));
                    });
                });
                body.row(20.0, |mut row| {
                    row.col(|ui| {
                        ui.label("Last avg time per sample");
                    });
                    row.col(|ui| {
                        ui.label(format!("{} ms", self.render_info.last_avg_time_per_sample));
                    });
                });
            });
    }
}

impl eframe::App for MirrorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut style: egui::Style = (*ctx.style()).clone();
        style.spacing.item_spacing.y = 6.0;
        ctx.set_style(style);

        // Force a repaint every second
        ctx.request_repaint_after(Duration::from_secs_f32(0.2));

        if ctx.input(|i| i.key_pressed(Key::Space)) {
            self.enable_side_panel = !self.enable_side_panel;
        }

        // Build ui
        egui::CentralPanel::default().show(ctx, |ui| {
            // Render image to background
            self.show_render_image(ui);
        });

        if self.enable_side_panel {
            let frame = egui::Frame {
                fill: egui::Color32::from_rgba_unmultiplied(20, 20, 20, 230),
                inner_margin: Margin::same(10i8),
                ..Default::default()
            };

            egui::SidePanel::left("side_panel")
                .frame(frame)
                .show(ctx, |ui| {
                    self.show_network(ui);
                    ui.separator();

                    self.show_rendering(ui);
                    ui.separator();

                    self.show_render_info(ui);
                });
        }
    }
}
