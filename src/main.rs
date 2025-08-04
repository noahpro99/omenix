use std::{
    sync::{Arc, Mutex, mpsc},
    thread,
};

use gtk::traits::GtkSettingsExt;
use tracing::{debug, error, info, warn};
use tray_icon::{
    TrayIconBuilder, TrayIconEvent,
    menu::{Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem},
};

mod fans;
use fans::{AppState, FanStatus};

// Define menu IDs as constants
const FAN_MAX_ID: &str = "fan_max";
const FAN_AUTO_ID: &str = "fan_auto";
const FAN_BIOS_ID: &str = "fan_bios";
const QUIT_ID: &str = "quit";

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

    // Create menu items and get their IDs
    let (menu, fan_max_id, fan_auto_id, fan_bios_id, quit_id) =
        create_menu_with_ids(app_state.clone());

    // Make tray_icon and keep it on the main thread
    let mut tray_icon = TrayIconBuilder::new()
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
    let (tx_menu_update, rx_menu_update) = mpsc::channel();

    // Create a timer thread for periodic status updates
    let tx_status_update = tx.clone();
    thread::spawn(move || {
        info!("Status update timer thread started");
        loop {
            thread::sleep(std::time::Duration::from_secs(30)); // Update every 30 seconds instead of 5
            debug!("Sending periodic status update");
            if tx_status_update.send(TrayMessage::UpdateStatus).is_err() {
                warn!("Failed to send status update - receiver may be closed");
                break;
            }
        }
    });

    info!("Menu item IDs retrieved");

    debug!(
        "Menu item IDs - Max: {}, Auto: {}, BIOS: {}, Quit: {}",
        FAN_MAX_ID, FAN_AUTO_ID, FAN_BIOS_ID, QUIT_ID
    );

    let tx_quit_clone = tx_quit.clone();
    thread::spawn(move || {
        info!("Menu event handler thread started");
        while let Ok(event) = menu_channel.recv() {
            debug!("Received menu event with ID: {:?}", event.id());
            let event_id_str = event.id().0.as_str(); // Get the string representation of the ID
            match event_id_str {
                FAN_MAX_ID => {
                    info!("Max Performance menu item clicked");
                    let _ = tx.send(TrayMessage::FansMax);
                }
                FAN_AUTO_ID => {
                    info!("Auto Control menu item clicked");
                    let _ = tx.send(TrayMessage::FansAuto);
                }
                FAN_BIOS_ID => {
                    info!("BIOS Default menu item clicked");
                    let _ = tx.send(TrayMessage::FansBios);
                }
                QUIT_ID => {
                    info!("Quit menu item clicked");
                    let _ = tx.send(TrayMessage::Exit);
                    let _ = tx_quit_clone.send(());
                }
                _ => {
                    warn!("Unknown menu event received: {:?}", event.id());
                }
            }
        }
        warn!("Menu event handler thread ended");
    });

    // Handle messages on the main thread
    let app_state_for_handler = app_state.clone();
    let tx_menu_update_clone = tx_menu_update.clone();
    thread::spawn(move || {
        info!("Message handler thread started");
        let mut last_status = String::new(); // Track the last status to avoid unnecessary updates
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
                    if let Err(e) =
                        fans::set_fan_status(app_state_for_handler.clone(), FanStatus::Max)
                    {
                        error!("Failed to set max fan mode: {}", e);
                    } else {
                        info!("✓ Fan mode set to: Max Performance");
                        debug!(
                            "Current app state: {:?}",
                            app_state_for_handler.lock().unwrap()
                        );
                        // Signal menu update
                        let _ = tx_menu_update_clone.send(());
                    }
                }
                TrayMessage::FansAuto => {
                    info!("Setting fans to Auto Control...");
                    if let Err(e) =
                        fans::set_fan_status(app_state_for_handler.clone(), FanStatus::Auto)
                    {
                        error!("Failed to set auto fan mode: {}", e);
                    } else {
                        info!(
                            "✓ Fan mode set to: Auto Control (will switch between Max/BIOS based on temperature)"
                        );
                        debug!(
                            "Current app state: {:?}",
                            app_state_for_handler.lock().unwrap()
                        );
                        // Signal menu update
                        let _ = tx_menu_update_clone.send(());
                    }
                }
                TrayMessage::FansBios => {
                    info!("Setting fans to BIOS Default...");
                    if let Err(e) =
                        fans::set_fan_status(app_state_for_handler.clone(), FanStatus::Bios)
                    {
                        error!("Failed to set bios fan mode: {}", e);
                    } else {
                        info!("✓ Fan mode set to: BIOS Default");
                        debug!(
                            "Current app state: {:?}",
                            app_state_for_handler.lock().unwrap()
                        );
                        // Signal menu update
                        let _ = tx_menu_update_clone.send(());
                    }
                }
                TrayMessage::UpdateStatus => {
                    let current_status = fans::fan_status_string(app_state_for_handler.clone());
                    debug!("Current status: {}", current_status);

                    // Only update the menu if the status has changed
                    if current_status != last_status {
                        info!(
                            "Status changed from '{}' to '{}', updating menu",
                            last_status, current_status
                        );
                        last_status = current_status.clone();
                        // Signal menu update
                        let _ = tx_menu_update_clone.send(());
                    } else {
                        debug!("Status unchanged, skipping menu update");
                    }

                    debug!(
                        "Current app state: {:?}",
                        app_state_for_handler.lock().unwrap()
                    );
                    info!("Current status: {}", current_status);
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

    info!("Starting main event loop");

    // Main thread loop to handle menu updates and GTK events
    loop {
        // Process GTK events
        while gtk::events_pending() {
            gtk::main_iteration();
        }

        // Check for menu update signals (non-blocking)
        if rx_menu_update.try_recv().is_ok() {
            info!("Updating tray icon menu...");
            let (new_menu, _, _, _, _) = create_menu_with_ids(app_state.clone());
            tray_icon.set_menu(Some(Box::new(new_menu)));
            info!("Tray icon menu updated successfully");
        }

        // Small sleep to prevent busy waiting
        thread::sleep(std::time::Duration::from_millis(50));
    }
}

fn create_menu_with_ids(
    app_state: Arc<Mutex<AppState>>,
) -> (
    Menu,
    tray_icon::menu::MenuId,
    tray_icon::menu::MenuId,
    tray_icon::menu::MenuId,
    tray_icon::menu::MenuId,
) {
    // Create menu items with consistent IDs
    let fan_max_id = MenuId::new(FAN_MAX_ID);
    let fan_auto_id = MenuId::new(FAN_AUTO_ID);
    let fan_bios_id = MenuId::new(FAN_BIOS_ID);
    let quit_id = MenuId::new(QUIT_ID);

    let fan_status = MenuItem::new(
        fans::fan_status_string(app_state.clone()).as_str(),
        false,
        None,
    );
    let separator1 = PredefinedMenuItem::separator();
    let fan_max = MenuItem::with_id(fan_max_id.clone(), "Fans Max", true, None);
    let fan_auto = MenuItem::with_id(fan_auto_id.clone(), "Fans Auto", true, None);
    let fan_bios = MenuItem::with_id(fan_bios_id.clone(), "Fans BIOS", true, None);
    let separator2 = PredefinedMenuItem::separator();
    let quit = MenuItem::with_id(quit_id.clone(), "Quit", true, None);

    let menu = Menu::with_items(&[
        &fan_status,
        &separator1,
        &fan_max,
        &fan_auto,
        &fan_bios,
        &separator2,
        &quit,
    ])
    .expect("Failed to create menu");

    (menu, fan_max_id, fan_auto_id, fan_bios_id, quit_id)
}
