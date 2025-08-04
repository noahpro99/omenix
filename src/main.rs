use std::sync::{Arc, Mutex, mpsc};
use tracing::{error, info};
use gtk::traits::GtkSettingsExt;

mod fans;
mod auth;
mod tray;

use fans::{AppState, FanStatus};
use tray::{TrayManager, TrayMessage};

fn main() {
    // Initialize tracing subscriber for structured logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "omenix=debug,warn".into()),
        )
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .init();

    info!("Starting Omenix Fan Control Application");

    gtk::init().expect("Failed to initialize GTK.");
    let Some(settings) = gtk::Settings::default() else {
        panic!("Failed to get default GTK settings.");
    };
    settings.set_gtk_application_prefer_dark_theme(true);

    let app_state = Arc::new(Mutex::new(AppState::new()));
    info!("App state initialized: {:?}", app_state.lock().unwrap());

    // Start fan control background thread
    fans::start_fan_control_thread(app_state.clone());

    // Create tray manager
    let mut tray_manager = TrayManager::new(app_state.clone())
        .expect("Failed to create tray manager");

    let (tx, rx) = mpsc::channel();
    let (tx_quit, rx_quit) = mpsc::channel();

    // Handle messages
    let app_state_for_handler = app_state.clone();
    let tx_quit_clone = tx_quit.clone();
    std::thread::spawn(move || {
        info!("Message handler thread started");
        while let Ok(message) = rx.recv() {
            match message {
                TrayMessage::FansMax => {
                    info!("Setting fans to Max Performance...");
                    if let Err(e) = fans::set_fan_status(app_state_for_handler.clone(), FanStatus::Max) {
                        error!("Failed to set max fan mode: {}", e);
                    } else {
                        info!("✓ Fan mode set to: Max Performance");
                    }
                }
                TrayMessage::FansAuto => {
                    info!("Setting fans to Auto Control...");
                    if let Err(e) = fans::set_fan_status(app_state_for_handler.clone(), FanStatus::Auto) {
                        error!("Failed to set auto fan mode: {}", e);
                    } else {
                        info!("✓ Fan mode set to: Auto Control");
                    }
                }
                TrayMessage::FansBios => {
                    info!("Setting fans to BIOS Default...");
                    if let Err(e) = fans::set_fan_status(app_state_for_handler.clone(), FanStatus::Bios) {
                        error!("Failed to set bios fan mode: {}", e);
                    } else {
                        info!("✓ Fan mode set to: BIOS Default");
                    }
                }
                TrayMessage::UpdateStatus => {
                    let current_status = fans::fan_status_string(app_state_for_handler.clone());
                    info!("Current status: {}", current_status);
                }
                TrayMessage::Exit => {
                    info!("Exiting application...");
                    let _ = tx_quit_clone.send(());
                    std::process::exit(0);
                }
            }
        }
    });

    // Handle quit signal
    std::thread::spawn(move || {
        info!("Quit signal handler thread started");
        let _ = rx_quit.recv();
        info!("Received quit signal, exiting application");
        std::process::exit(0);
    });

    info!("Starting main event loop");

    // Start tray manager event loop
    tray_manager.start_event_loop(tx)
        .expect("Failed to start tray manager event loop");
}


