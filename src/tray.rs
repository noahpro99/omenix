use std::sync::mpsc;
use tracing::{debug, info, warn};
use tray_icon::{
    TrayIconBuilder, TrayIconEvent,
    menu::{Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem},
};

use crate::types::{FanMode, TrayMessage};

// Define menu IDs as constants
const FAN_MAX_ID: &str = "fan_max";
const FAN_AUTO_ID: &str = "fan_auto";
const FAN_BIOS_ID: &str = "fan_bios";
const QUIT_ID: &str = "quit";

pub struct TrayManager {
    tray_icon: tray_icon::TrayIcon,
}

impl TrayManager {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
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

        // Create static menu (no more constant recreation)
        let menu = create_static_menu();

        let tray_icon = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("Omenix - Fan Control")
            .with_icon(icon)
            .build()
            .expect("Failed to create tray icon");

        info!("System tray icon created successfully");

        Ok(Self { tray_icon })
    }

    pub fn start_event_loop(
        &mut self,
        tx: mpsc::Sender<TrayMessage>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let menu_channel = MenuEvent::receiver();
        let _tray_channel = TrayIconEvent::receiver();

        // Handle menu events in a single thread
        std::thread::spawn(move || {
            info!("Menu event handler thread started");
            while let Ok(event) = menu_channel.recv() {
                debug!("Received menu event with ID: {:?}", event.id());
                let event_id_str = event.id().0.as_str();
                match event_id_str {
                    FAN_MAX_ID => {
                        info!("Max Performance menu item clicked");
                        let _ = tx.send(TrayMessage::SetMode(FanMode::Max));
                    }
                    FAN_AUTO_ID => {
                        info!("Auto Control menu item clicked");
                        let _ = tx.send(TrayMessage::SetMode(FanMode::Auto));
                    }
                    FAN_BIOS_ID => {
                        info!("BIOS Default menu item clicked");
                        let _ = tx.send(TrayMessage::SetMode(FanMode::Bios));
                    }
                    QUIT_ID => {
                        info!("Quit menu item clicked");
                        let _ = tx.send(TrayMessage::Exit);
                    }
                    _ => {
                        warn!("Unknown menu event received: {:?}", event.id());
                    }
                }
            }
            warn!("Menu event handler thread ended");
        });

        info!("Starting tray manager event loop");

        // Simple GTK main loop - no more busy waiting or constant menu updates
        loop {
            gtk::main_iteration_do(true); // Block until next event
        }
    }
}

fn create_static_menu() -> Menu {
    // Create menu items with consistent IDs
    let fan_max_id = MenuId::new(FAN_MAX_ID);
    let fan_auto_id = MenuId::new(FAN_AUTO_ID);
    let fan_bios_id = MenuId::new(FAN_BIOS_ID);
    let quit_id = MenuId::new(QUIT_ID);

    let fan_max = MenuItem::with_id(fan_max_id, "🔥 Max Performance", true, None);
    let fan_auto = MenuItem::with_id(fan_auto_id, "🤖 Auto Control", true, None);
    let fan_bios = MenuItem::with_id(fan_bios_id, "💻 BIOS Default", true, None);
    let separator = PredefinedMenuItem::separator();
    let quit = MenuItem::with_id(quit_id, "Quit", true, None);

    Menu::with_items(&[&fan_max, &fan_auto, &fan_bios, &separator, &quit])
        .expect("Failed to create menu")
}
