use std::sync::{Arc, Mutex, mpsc};
use tracing::{debug, info, warn};
use tray_icon::{
    TrayIconBuilder, TrayIconEvent,
    menu::{Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem},
};

use crate::fans::AppState;

// Define menu IDs as constants
const FAN_MAX_ID: &str = "fan_max";
const FAN_AUTO_ID: &str = "fan_auto";
const FAN_BIOS_ID: &str = "fan_bios";
const QUIT_ID: &str = "quit";

#[derive(Debug)]
pub enum TrayMessage {
    FansMax,
    FansAuto,
    FansBios,
    UpdateStatus,
    Exit,
}

pub struct TrayManager {
    tray_icon: tray_icon::TrayIcon,
    app_state: Arc<Mutex<AppState>>,
}

impl TrayManager {
    pub fn new(app_state: Arc<Mutex<AppState>>) -> Result<Self, Box<dyn std::error::Error>> {
        let path = "assets/icon.png";

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
        let (menu, _, _, _, _) = create_menu_with_ids(app_state.clone());

        // Make tray_icon and keep it on the main thread
        let tray_icon = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("Omenix - Fan Control")
            .with_icon(icon)
            .build()
            .expect("Failed to create tray icon");

        info!("System tray icon created successfully");

        Ok(Self {
            tray_icon,
            app_state,
        })
    }

    pub fn start_event_loop(&mut self, tx: mpsc::Sender<TrayMessage>) -> Result<(), Box<dyn std::error::Error>> {
        let menu_channel = MenuEvent::receiver();
        let _tray_channel = TrayIconEvent::receiver();
        let (tx_menu_update, rx_menu_update) = mpsc::channel();

        // Handle menu events
        let tx_menu = tx.clone();
        std::thread::spawn(move || {
            info!("Menu event handler thread started");
            while let Ok(event) = menu_channel.recv() {
                debug!("Received menu event with ID: {:?}", event.id());
                let event_id_str = event.id().0.as_str();
                match event_id_str {
                    FAN_MAX_ID => {
                        info!("Max Performance menu item clicked");
                        let _ = tx_menu.send(TrayMessage::FansMax);
                    }
                    FAN_AUTO_ID => {
                        info!("Auto Control menu item clicked");
                        let _ = tx_menu.send(TrayMessage::FansAuto);
                    }
                    FAN_BIOS_ID => {
                        info!("BIOS Default menu item clicked");
                        let _ = tx_menu.send(TrayMessage::FansBios);
                    }
                    QUIT_ID => {
                        info!("Quit menu item clicked");
                        let _ = tx_menu.send(TrayMessage::Exit);
                    }
                    _ => {
                        warn!("Unknown menu event received: {:?}", event.id());
                    }
                }
            }
            warn!("Menu event handler thread ended");
        });

        // Create a timer thread for periodic status updates
        let tx_status_update = tx.clone();
        std::thread::spawn(move || {
            info!("Status update timer thread started");
            loop {
                std::thread::sleep(std::time::Duration::from_secs(30));
                debug!("Sending periodic status update");
                if tx_status_update.send(TrayMessage::UpdateStatus).is_err() {
                    warn!("Failed to send status update - receiver may be closed");
                    break;
                }
            }
        });

        // Signal menu updates
        let tx_menu_update_clone = tx_menu_update.clone();
        let tx_signal = tx.clone();
        std::thread::spawn(move || {
            while tx_signal.send(TrayMessage::UpdateStatus).is_ok() {
                std::thread::sleep(std::time::Duration::from_secs(1));
                let _ = tx_menu_update_clone.send(());
            }
        });

        info!("Starting tray manager event loop");

        // Main thread loop to handle menu updates and GTK events
        loop {
            // Process GTK events
            while gtk::events_pending() {
                gtk::main_iteration();
            }

            // Check for menu update signals (non-blocking)
            if rx_menu_update.try_recv().is_ok() {
                info!("Updating tray icon menu...");
                let (new_menu, _, _, _, _) = create_menu_with_ids(self.app_state.clone());
                self.tray_icon.set_menu(Some(Box::new(new_menu)));
                debug!("Tray icon menu updated successfully");
            }

            // Small sleep to prevent busy waiting
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
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
        crate::fans::fan_status_string(app_state.clone()).as_str(),
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
