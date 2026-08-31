#![windows_subsystem = "windows"]

mod app;
mod config;
mod network;
mod transfer;
mod ui;

use app::LankerApp;
use config::Config;

fn main() -> eframe::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("lanker=info")
        .with_target(false)
        .init();

    tracing::info!("Starting Lanker...");

    let config = Config::load();

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()
        .expect("Failed to create tokio runtime");

    let handle = runtime.handle().clone();

    let _guard = runtime.enter();

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Lanker — LAN File Transfer")
            .with_inner_size([920.0, 620.0])
            .with_min_inner_size([700.0, 400.0])
            .with_drag_and_drop(true),
        ..Default::default()
    };

    eframe::run_native(
        "Lanker",
        native_options,
        Box::new(move |cc| {
            Ok(Box::new(LankerApp::new(cc, config, handle)))
}),
    )
}
