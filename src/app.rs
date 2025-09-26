use eframe::egui;
use egui_extras::{Column, TableBuilder};
use tracing::trace;

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
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Mirror Network");

            TableBuilder::new(ui)
                .striped(true)
                .resizable(false)
                .cell_layout(egui::Layout::left_to_right(egui::Align::Min))
                .columns(Column::remainder(), 2)
                .body(|mut body| {
                    for i in 0..200 {
                        body.row(20.0, |mut row| {
                            row.col(|ui| {
                                ui.label(format!("peer{i}"));
                            });
                            row.col(|ui| {
                                ui.label("127.0.0.1:2021");
                            });
                        });
                    }
                });
        });
    }
}
