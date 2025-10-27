use chrono::Local;
use tokio;
use tracing::info;
use tracing_subscriber::fmt::{format::Writer, time::FormatTime};

///////////////////////////////////////////////////////////////////////////////
use eframe::egui::{
    self, Color32, ColorImage, DragValue, Grid, Key, Margin, RichText, TextureHandle, Ui,
    load::Bytes,
};
use egui_extras::{Column, TableBuilder};

#[derive(Default)]
pub struct RenderInfo {
    pub total_samples: usize,
    pub total_time: u128,
    pub last_samples: usize,
    pub last_time: u128,
    pub total_avg_time_per_sample: u128,
    pub last_avg_time_per_sample: u128,
}

#[derive(Default)]
struct TestApp {
    // Ui data
    enable_side_panel: bool,
    // Rendering
    progressive_rendering: bool,
    samples_per_pixel: usize,
    framebuffer_size: (usize, usize),
    // Render info
    render_info: RenderInfo,
}

impl TestApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Customize egui here with cc.egui_ctx.set_fonts and cc.egui_ctx.set_visuals.
        // Restore app state using cc.storage (requires the "persistence" feature).
        // Use the cc.gl (a glow::Context) to create graphics shaders and buffers that you can use
        // for e.g. egui::PaintCallback.
        Self {
            enable_side_panel: true,
            progressive_rendering: false,
            samples_per_pixel: 20,
            framebuffer_size: (400, 400),
            ..Default::default()
        }
    }

    fn show_network(&mut self, ui: &mut egui::Ui) {
        ui.heading(RichText::new("Network").color(Color32::LIGHT_GRAY));

        // NOTE: Since Im using try_lock to get peers info to avoid blocking
        // ui task, I use a Vec to cache the info when its not possible to get
        // the lock guard.
        let cached_peers_info: Vec<(Option<String>, String)> = vec![
            (Some("foo".into()), "127.0.0.1:2020".into()),
            (Some("bar".into()), "127.0.0.1:2021".into()),
            (None, "127.0.0.1:2022".into()),
        ];

        if cached_peers_info.len() == 0 {
            ui.label("No connected peers.");
        } else {
            TableBuilder::new(ui)
                .id_salt("network_table")
                .striped(true)
                .resizable(false)
                .cell_layout(egui::Layout::left_to_right(egui::Align::Min))
                .columns(Column::remainder(), 2)
                .body(|mut body| {
                    for (i, (name, address)) in cached_peers_info.iter().enumerate() {
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

    fn show_rendering(&mut self, ui: &mut egui::Ui) {
        ui.heading(RichText::new("Rendering").color(Color32::LIGHT_GRAY));

        let is_rendering = false;
        // Render button
        let render_button = ui.add_enabled(!is_rendering, |ui: &mut Ui| {
            TableBuilder::new(ui)
                .id_salt("rendering_table")
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
                                log::info!("Reized framebuffer");
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
            log::info!("Spawn render task");
            // self.spawn_render_task();
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
            log::info!("Saving image");
            // self.save_render_image(
            //     format!("render_{}.png", Local::now().format("%Y%m%d_%H%M%S")).as_str(),
            // );
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

impl eframe::App for TestApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        let mut style: egui::Style = (*ctx.style()).clone();
        style.spacing.item_spacing.y = 6.0;
        ctx.set_style(style);

        if ctx.input(|i| i.key_pressed(Key::Space)) {
            self.enable_side_panel = !self.enable_side_panel;
        }

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
///////////////////////////////////////////////////////////////////////////////

// When compiling to web using trunk:
#[cfg(target_arch = "wasm32")]
pub fn main() {
    use eframe::wasm_bindgen::JsCast as _;
    // Initialize logger.
    tracing_wasm::set_as_global_default();

    eframe::WebLogger::init(log::LevelFilter::Debug).ok();
    tracing::info!("Foo");

    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();

    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        let document = web_sys::window()
            .expect("No window")
            .document()
            .expect("No document");

        let canvas = document
            .get_element_by_id("the_canvas_id")
            .expect("Failed to find the_canvas_id")
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .expect("the_canvas_id was not a HtmlCanvasElement");

        let start_result = eframe::WebRunner::new()
            .start(
                canvas,
                web_options,
                Box::new(|cc| Ok(Box::new(TestApp::new(cc)))),
            )
            .await;
    });
}
