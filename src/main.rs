use std::{sync::{mpsc, Arc, Mutex}, thread};

use gtk::traits::GtkSettingsExt;
use tray_icon::{menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem}, TrayIconBuilder, TrayIconEvent};
use tracing::{debug, error, info, warn};

mod fans;
use fans::{AppState, FanStatus};

#[derive(Debug)]
pub enum TrayMessage {
    PerformanceMode,
    DefaultMode,
    FansMax,
    FansAuto,
    FansBios,
    UpdateStatus,
    Exit,
}


fn main() {
    // Initialize tracing subscriber for structured logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "omenix=debug,warn".into())
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
    debug!("GTK initialized with dark theme preference");
    
    let path = "assets/icon.png";

    let app_state = Arc::new(Mutex::new(AppState::new()));
    info!("App state initialized: {:?}", app_state.lock().unwrap());
    
    fans::start_fan_control_thread(app_state.clone());

    let (icon_rgba, icon_width, icon_height) = {
        debug!("Loading icon from path: {}", path);
        let image = image::open(path)
            .expect("Failed to open icon path")
            .into_rgba8();
        let (width, height) = image.dimensions();
        let rgba = image.into_raw();
        debug!("Icon loaded successfully: {}x{}", width, height);
        (rgba, width, height)
    };
    let icon = tray_icon::Icon::from_rgba(icon_rgba, icon_width, icon_height)
        .expect("Failed to open icon");

    let fan_status = MenuItem::new(fans::fan_status_string(app_state.clone()).as_str(), false, None);
    debug!("Created fan status menu item: {}", fans::fan_status_string(app_state.clone()));
    let separator1 = PredefinedMenuItem::separator();
    let fan_max = MenuItem::new("Fans Max", true, None);
    let fan_auto = MenuItem::new("Fans Auto", true, None);
    let fan_bios = MenuItem::new("Fans BIOS", true, None);
    let separator2 = PredefinedMenuItem::separator();
    let quit = MenuItem::new("Quit", true, None);

    let menu = Menu::with_items(&[
        &fan_status,
        &separator1,
        &fan_max,
        &fan_auto,
        &fan_bios,
        &separator2,
        &quit,
    ]).expect("Failed to create menu");

    let _tray_icon = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("Omenix - Fan Control")
        .with_icon(icon)
        .build()
        .expect("Failed to create tray icon");

    info!("System tray icon created successfully");

    let menu_channel = MenuEvent::receiver();
    let _tray_channel = TrayIconEvent::receiver();

    let (tx, rx) = mpsc::channel();
    let (tx_quit, rx_quit) = mpsc::channel();

    // Create a timer thread for periodic status updates
    let tx_status_update = tx.clone();
    thread::spawn(move || {
        info!("Status update timer thread started");
        loop {
            thread::sleep(std::time::Duration::from_secs(5)); // Update every 5 seconds
            debug!("Sending periodic status update");
            if tx_status_update.send(TrayMessage::UpdateStatus).is_err() {
                warn!("Failed to send status update - receiver may be closed");
                break;
            }
        }
    });

    let clone_fan_max_id = fan_max.id().clone();
    let clone_fan_auto_id = fan_auto.id().clone();
    let clone_fan_bios_id = fan_bios.id().clone();
    let quit_id = quit.id().clone();

    debug!("Menu item IDs - Max: {:?}, Auto: {:?}, BIOS: {:?}, Quit: {:?}", 
           clone_fan_max_id, clone_fan_auto_id, clone_fan_bios_id, quit_id);

    let tx_quit_clone = tx_quit.clone();
    thread::spawn(move || {
        info!("Menu event handler thread started");
        while let Ok(event) = menu_channel.recv() {
            debug!("Received menu event with ID: {:?}", event.id());
            if *event.id() == clone_fan_max_id {
                info!("Max Performance menu item clicked");
                let _ = tx.send(TrayMessage::FansMax);
            } else if *event.id() == clone_fan_auto_id {
                info!("Auto Control menu item clicked");
                let _ = tx.send(TrayMessage::FansAuto);
            } else if *event.id() == clone_fan_bios_id {
                info!("BIOS Default menu item clicked");
                let _ = tx.send(TrayMessage::FansBios);
            } else if *event.id() == quit_id {
                info!("Quit menu item clicked");
                let _ = tx.send(TrayMessage::Exit);
                let _ = tx_quit_clone.send(());
            } else {
                warn!("Unknown menu event received: {:?}", event.id());
            }
        }
        warn!("Menu event handler thread ended");
    });

    // setup handler for rx
    let app_state_for_handler = app_state.clone();
    thread::spawn(move || {
        info!("Message handler thread started");
        while let Ok(message) = rx.recv() {
            debug!("Processing message: {:?}", message);
            match message {
                TrayMessage::PerformanceMode => {
                    info!("Switching to Performance Mode...");
                }
                TrayMessage::DefaultMode => {
                    info!("Switching to Default Mode...");
                }
                TrayMessage::FansMax => {
                    info!("Setting fans to Max Performance...");
                    if let Err(e) = fans::set_fan_status(app_state_for_handler.clone(), FanStatus::Max) {
                        error!("Failed to set max fan mode: {}", e);
                    } else {
                        info!("✓ Fan mode set to: Max Performance");
                        debug!("Current app state: {:?}", app_state_for_handler.lock().unwrap());
                    }
                }
                TrayMessage::FansAuto => {
                    info!("Setting fans to Auto Control...");
                    if let Err(e) = fans::set_fan_status(app_state_for_handler.clone(), FanStatus::Auto) {
                        error!("Failed to set auto fan mode: {}", e);
                    } else {
                        info!("✓ Fan mode set to: Auto Control (will switch between Max/BIOS based on temperature)");
                        debug!("Current app state: {:?}", app_state_for_handler.lock().unwrap());
                    }
                }
                TrayMessage::FansBios => {
                    info!("Setting fans to BIOS Default...");
                    if let Err(e) = fans::set_fan_status(app_state_for_handler.clone(), FanStatus::Bios) {
                        error!("Failed to set bios fan mode: {}", e);
                    } else {
                        info!("✓ Fan mode set to: BIOS Default");
                        debug!("Current app state: {:?}", app_state_for_handler.lock().unwrap());
                    }
                }
                TrayMessage::UpdateStatus => {
                    info!("Updating status...");
                    debug!("Current status: {}", fans::fan_status_string(app_state_for_handler.clone()));
                }
                TrayMessage::Exit => {
                    info!("Exiting application...");
                    std::process::exit(0);
                }
            }
        }
        warn!("Message handler thread ended");
    });

    // Handle quit signal
    thread::spawn(move || {
        info!("Quit signal handler thread started");
        let _ = rx_quit.recv();
        info!("Received quit signal, exiting application");
        std::process::exit(0);
    });

    info!("Starting GTK main loop");
    gtk::main();
}