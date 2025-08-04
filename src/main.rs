use gtk::traits::GtkSettingsExt;
use std::sync::mpsc;
use tracing::{error, info};

mod client;
mod tray;
mod types;

use client::DaemonClient;
use tray::TrayManager;
use types::TrayMessage;

fn main() {
    // Initialize tracing subscriber for structured logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
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

    // Create daemon client
    let client = DaemonClient::new();

    // Check if daemon is running
    if !client.is_daemon_running() {
        error!("Cannot connect to daemon. Please make sure 'omenix-daemon' is running as root.");
        error!("Run: sudo omenix-daemon");
        std::process::exit(1);
    }

    info!("Connected to daemon successfully");

    // Create tray manager
    let mut tray_manager = TrayManager::new().expect("Failed to create tray manager");

    let (tx, rx) = mpsc::channel();
    let (tx_quit, rx_quit) = mpsc::channel();

    // Handle messages
    let tx_quit_clone = tx_quit.clone();
    std::thread::spawn(move || {
        info!("Message handler thread started");
        let daemon_client = DaemonClient::new();

        while let Ok(message) = rx.recv() {
            match message {
                TrayMessage::SetMode(mode) => {
                    info!("Setting fan mode to: {}...", mode);
                    if let Err(e) = daemon_client.set_fan_mode(mode) {
                        error!("Failed to set fan mode: {}", e);
                    } else {
                        info!("✓ Fan mode set to: {}", mode);
                    }
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
    tray_manager
        .start_event_loop(tx)
        .expect("Failed to start tray manager event loop");
}
