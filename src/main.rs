#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
//! Desktop Application Entry Point (Windows 11 & macOS) — DAX - Data Analysis Exchange (v0.2).

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
            .with_title("DAX - Data Analysis Exchange (v0.3) — Equities, FX (DXY), Global Rates & Macroeconomics")
            .with_inner_size([1440.0, 900.0])
            .with_min_inner_size([1024.0, 640.0]),
        ..Default::default()
    };

    eframe::run_native(
        "dax_data_analysis_exchange_v02",
        native_options,
        Box::new(|cc| Ok(Box::new(DaxEconomiaApp::new(cc)))),
    )
}
