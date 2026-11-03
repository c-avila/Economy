#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
//! Ponto de entrada da aplicação Desktop para Windows 11 (`dax_economia_desktop`).

mod app;
mod bridge;
mod config;
mod models;
mod theme;

use app::DaxEconomiaApp;
use eframe::egui;

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("DAX Economia — Ações, Moedas (DXY), Juros Globais & Indicadores Macroeconômicos")
            .with_inner_size([1440.0, 880.0])
            .with_min_inner_size([1100.0, 680.0]),
        ..Default::default()
    };

    eframe::run_native(
        "dax_economia_desktop",
        native_options,
        Box::new(|cc| Ok(Box::new(DaxEconomiaApp::new(cc)))),
    )
}

