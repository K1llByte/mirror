use std::time::Duration;

use eframe::egui;
use egui_extras::{Column, TableBuilder};
use tracing::{debug, info, trace};

use crate::peer::PeerTable;

#[derive(Default)]
pub struct MirrorApp {
    peer_table: PeerTable,
}

impl MirrorApp {
    pub fn new(peer_table: PeerTable) -> Self {
        Self { peer_table }
    }
}

impl eframe::App for MirrorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Force a repaint every second
        ctx.request_repaint_after(Duration::from_secs(1));

        // Build ui
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Mirror Network");

            if let Ok(peer_table) = self.peer_table.try_lock() {
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
        });
    }
}
