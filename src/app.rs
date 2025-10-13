use std::{
    sync::{Arc, atomic::AtomicUsize},
    time::Duration,
};

use eframe::egui::{
    self, Color32, ColorImage, DragValue, Key, RichText, TextureHandle, Ui, load::Bytes,
};
use egui_extras::{Column, TableBuilder};
use glam::Vec3;
use tokio::{runtime::Runtime, sync::RwLock, task::JoinHandle};

use crate::{
    accum_image::AccumulatedImage,
    image::Image,
    renderer::{self, Renderer},
    scene::Scene,
};

pub struct MirrorApp {
    // Backend data
    runtime: Runtime,
    renderer: Arc<Renderer>,
    render_image: Arc<RwLock<AccumulatedImage>>,
    scene: Arc<Scene>,

    // Ui data
    present_framebuffer: bool,
    enable_side_panel: bool,
    texture: Option<egui::TextureHandle>,
    render_join_handle: Option<JoinHandle<()>>,
    progressive_rendering: bool,
    samples_per_pixel: usize,
    framebuffer_size: (usize, usize),
    cached_peers_info: Vec<(Option<String>, String)>,
}

impl MirrorApp {
    pub fn new(runtime: Runtime, renderer: Arc<Renderer>, scene: Arc<Scene>) -> Self {
        let framebuffer_size = (1280, 720);
        Self {
            // Backend data
            runtime,
            renderer,
            render_image: Arc::new(RwLock::new(AccumulatedImage::new(framebuffer_size))),
            scene,
            // Ui data
            present_framebuffer: false,
            enable_side_panel: true,
            texture: None,
            render_join_handle: None,
            progressive_rendering: false,
            samples_per_pixel: 1,
            framebuffer_size,
            cached_peers_info: vec![],
        }
    }

    fn spawn_render_task(&mut self) {
        self.render_join_handle = Some(self.runtime.spawn(renderer::render_task(
            self.renderer.clone(),
            self.render_image.clone(),
            self.scene.clone(),
            self.samples_per_pixel,
        )));
    }

    fn show_render_image(&mut self, ui: &mut egui::Ui) {
        // FIXME: Turn blocking lock into try_lock to avoid ui blocking

        let texture: &TextureHandle = if self
            .render_join_handle
            .as_ref()
            .is_some_and(|fut| fut.is_finished())
            || self.present_framebuffer
        {
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

    fn show_connected_peers(&mut self, ui: &mut egui::Ui) {
        // NOTE: Since Im using try_lock to get peers info to avoid blocking
        // ui task, I use a Vec to cache the info when its not possible to get
        // the lock guard.
        if let Ok(peer_table_guard) = self.renderer.peer_table.try_read() {
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
}

impl eframe::App for MirrorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut style: egui::Style = (*ctx.style()).clone();
        style.spacing.item_spacing.y = 6.0;
        ctx.set_style(style);

        // Force a repaint every second
        ctx.request_repaint_after(Duration::from_secs_f32(0.3));

        if ctx.input(|i| i.key_pressed(Key::Space)) {
            self.enable_side_panel = !self.enable_side_panel;
        }

        // Build ui
        egui::CentralPanel::default().show(ctx, |ui| {
            // Render image to background
            self.show_render_image(ui);
        });

        if self.enable_side_panel {
            egui::SidePanel::left("side_panel").show(ctx, |ui| {
                ui.heading("Mirror");
                ui.separator();
                self.show_connected_peers(ui);
                ui.separator();

                let is_rendering = self
                    .render_join_handle
                    .as_ref()
                    .is_some_and(|fut| !fut.is_finished());
                let render_button = ui.add_enabled(!is_rendering, |ui: &mut Ui| {
                    // Framebuffer size Slider
                    ui.horizontal(|ui| {
                        ui.label("Framebuffer size");
                        let fb_width_drag = ui.add(DragValue::new(&mut self.framebuffer_size.0));
                        let fb_height_drag = ui.add(DragValue::new(&mut self.framebuffer_size.1));
                        if fb_width_drag.changed() || fb_height_drag.changed() {
                            self.render_image
                                .blocking_write()
                                .resize(self.framebuffer_size);
                        }
                    });

                    // Samples per pixel Slider
                    ui.horizontal(|ui| {
                        ui.label("Samples per pixel");
                        ui.add(DragValue::new(&mut self.samples_per_pixel));
                    });

                    // Progressive rendering checkbox
                    ui.checkbox(&mut self.progressive_rendering, "Progressive Rendering");

                    ui.separator();

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

                // let stop_button = ui.add_enabled(is_rendering, |ui: &mut Ui| {
                //     ui.add_sized(
                //         [ui.available_width(), 30.0],
                //         egui::Button::new("Stop").fill(Color32::from_rgb(100, 40, 40)),
                //     )
                // });

                if render_button.clicked() {
                    self.spawn_render_task();
                }

                // Clear Button
                let clear_button =
                    ui.add_sized([ui.available_width(), 0.0], egui::Button::new("Clear"));
                if clear_button.clicked() {
                    if let Some(render_join_handle) = &self.render_join_handle {
                        render_join_handle.abort();
                        self.render_join_handle = None;
                    }
                    let clear_value = 20.0 / 255.0;
                    self.render_image.blocking_write().clear(Vec3::new(
                        clear_value,
                        clear_value,
                        clear_value,
                    ));
                    self.present_framebuffer = true;
                }
            });
        }
    }
}
