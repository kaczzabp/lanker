#![windows_subsystem = "windows"]

mod app;
mod config;
mod network;
mod transfer;
mod ui;

use app::LankerApp;
use config::Config;

/// Builds the window icon: a black "L" on an amber rounded square,
/// matching the logo tile drawn in the app sidebar.
fn build_icon() -> std::sync::Arc<egui::IconData> {
    const SIZE: u32 = 256;
    const RADIUS: f32 = 56.0;
    const AMBER: [u8; 3] = [0xF0, 0xA5, 0x00];

    let mut rgba = Vec::with_capacity((SIZE * SIZE * 4) as usize);
    for y in 0..SIZE {
        for x in 0..SIZE {
            let fx = x as f32 + 0.5;
            let fy = y as f32 + 0.5;
            let s = SIZE as f32;

            // Rounded-rectangle coverage with 1px anti-aliased edge.
            let nx = fx.clamp(RADIUS, s - RADIUS);
            let ny = fy.clamp(RADIUS, s - RADIUS);
            let dist = ((fx - nx).powi(2) + (fy - ny).powi(2)).sqrt();
            let coverage = (RADIUS - dist + 0.5).clamp(0.0, 1.0);
            if coverage <= 0.0 {
                rgba.extend_from_slice(&[0, 0, 0, 0]);
                continue;
            }

            // Bold "L": vertical bar + foot.
            let in_bar = fx >= 0.30 * s && fx < 0.48 * s && fy >= 0.20 * s && fy < 0.80 * s;
            let in_foot = fx >= 0.30 * s && fx < 0.72 * s && fy >= 0.62 * s && fy < 0.80 * s;
            let pixel = if in_bar || in_foot { [0x11, 0x11, 0x11] } else { AMBER };
            rgba.extend_from_slice(&[pixel[0], pixel[1], pixel[2], (coverage * 255.0) as u8]);
        }
    }

    std::sync::Arc::new(egui::IconData {
        rgba,
        width: SIZE,
        height: SIZE,
    })
}

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
            .with_drag_and_drop(true)
            .with_icon(build_icon()),
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
