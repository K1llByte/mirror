use std::{
    sync::{Arc, atomic::AtomicUsize},
    time::Duration,
};

use eframe::egui::{self, ColorImage, Key, TextureHandle, Ui, load::Bytes};
use egui_extras::{Column, TableBuilder};
use tokio::{runtime::Runtime, sync::Mutex, task::JoinHandle};

use crate::{
    image::Image,
    renderer::{self, Renderer},
    scene::Scene,
};

pub struct MirrorApp {
    // Backend data
    runtime: Runtime,
    renderer: Arc<Renderer>,
    render_image: Arc<Mutex<Image>>,
    scene: Arc<Scene>,

    // Ui data
    enable_side_panel: bool,
    texture: Option<egui::TextureHandle>,
    render_join_handle: Option<JoinHandle<()>>,
    progressive_rendering: bool,
}

impl MirrorApp {
    pub fn new(runtime: Runtime, renderer: Arc<Renderer>, scene: Arc<Scene>) -> Self {
        Self {
            // Backend data
            runtime,
            renderer,
            render_image: Arc::new(Mutex::new(Image::new((1280, 720)))),
            scene,
            // Ui data
            enable_side_panel: true,
            texture: None,
            render_join_handle: None,
            progressive_rendering: false,
        }
    }

    fn show_render_image(&mut self, ui: &mut egui::Ui) {
        let image_size: [usize; 2] = self.render_image.blocking_lock().size().into();

        let texture: &TextureHandle = if self
            .render_join_handle
            .as_ref()
            .is_some_and(|fut| fut.is_finished())
        {
            let image_bytes =
                Bytes::Shared(Arc::from(self.render_image.blocking_lock().to_bytes()));
            let image_data = ColorImage::from_rgb(image_size, image_bytes.as_ref());
            self.texture.replace(ui.ctx().load_texture(
                "render_image",
                image_data,
                Default::default(),
            ));

            self.render_join_handle = if self.progressive_rendering {
                Some(self.runtime.spawn(renderer::render_task(
                    self.renderer.clone(),
                    self.render_image.clone(),
                    self.scene.clone(),
                )))
            } else {
                None
            };

            self.texture.as_ref().unwrap()
        } else {
            self.texture.get_or_insert_with(|| {
                let image_bytes =
                    Bytes::Shared(Arc::from(vec![20u8; image_size[0] * image_size[1] * 3]));
                let image_data = ColorImage::from_rgb(image_size, image_bytes.as_ref());

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

                if let Ok(peer_table_guard) = self.renderer.peer_table.try_lock() {
                    if peer_table_guard.keys().len() == 0 {
                        ui.label("No connected peers.");
                    } else {
                        TableBuilder::new(ui)
                            .striped(true)
                            .resizable(false)
                            .cell_layout(egui::Layout::left_to_right(egui::Align::Min))
                            .columns(Column::remainder(), 2)
                            .body(|mut body| {
                                for (i, address) in peer_table_guard.keys().enumerate() {
                                    body.row(20.0, |mut row| {
                                        row.col(|ui| {
                                            ui.label(format!(
                                                "{}: {}",
                                                i + 1,
                                                peer_table_guard
                                                    .get(&address)
                                                    .unwrap()
                                                    .name
                                                    .clone()
                                                    .unwrap_or("<unnamed>".into())
                                            ));
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

                ui.checkbox(&mut self.progressive_rendering, "Progressive Rendering");

                ui.add_space(3.0);

                let is_rendering = self
                    .render_join_handle
                    .as_ref()
                    .is_some_and(|fut| !fut.is_finished());
                let render_button = ui.add_enabled(!is_rendering, |ui: &mut Ui| {
                    ui.add_sized(
                        [ui.available_width(), 0.0],
                        egui::Button::new(if is_rendering {
                            "Rendering ..."
                        } else {
                            "Render"
                        }),
                    )
                });
                if render_button.clicked() {
                    self.render_join_handle = Some(self.runtime.spawn(renderer::render_task(
                        self.renderer.clone(),
                        self.render_image.clone(),
                        self.scene.clone(),
                    )));
                }
            });
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            // Render image to background
            self.show_render_image(ui);
        });
    }
}
