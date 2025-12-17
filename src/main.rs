mod locale;
mod share;
mod ui;
mod worker;

use crate::ui::YtGUI;
use dotenv::dotenv;
use eframe::egui;

#[tokio::main]
async fn main() {
    dotenv().ok(); // Load environment variables from .env file

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder {
            min_inner_size: Some(egui::vec2(800.0, 600.0)),
            ..Default::default()
        },
        ..Default::default()
    };

    let app = eframe::run_native("", options, Box::new(|cc| Ok(Box::new(YtGUI::new(cc)))));

    if let Err(error) = app {
        eprintln!("Fehler beim Starten der App: {}", error);
    }
}
