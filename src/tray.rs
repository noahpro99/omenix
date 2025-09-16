use std::sync::{Arc, Mutex, mpsc};
use tracing::{debug, info, warn};
use tray_icon::{
    TrayIconBuilder, TrayIconEvent,
    menu::{Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem, Submenu},
};

use crate::client::DaemonClient;
use crate::types::{FanMode, PerformanceMode, SystemState, TrayMessage};

// Define menu IDs as constants
const FAN_MAX_ID: &str = "fan_max";
const FAN_AUTO_ID: &str = "fan_auto";
const FAN_BIOS_ID: &str = "fan_bios";

const PERF_BALANCED_ID: &str = "perf_balanced";
const PERF_PERFORMANCE_ID: &str = "perf_performance";

const QUIT_ID: &str = "quit";

pub struct TrayManager {
    tray_icon: tray_icon::TrayIcon,
    client: DaemonClient,
    cached_state: Arc<Mutex<Option<SystemState>>>,
}

impl TrayManager {
    fn get_icon_path() -> String {
        // Try environment variable first (set by Nix build)
        if let Ok(assets_dir) = std::env::var("OMENIX_ASSETS_DIR") {
            format!("{}/icon.png", assets_dir)
        }
        // Fallback to relative path for development
        else {
            "assets/icon.png".to_string()
        }
    }

    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let client = DaemonClient::new();
        let icon_path = Self::get_icon_path();

        let (icon_rgba, icon_width, icon_height) = {
            debug!("Loading icon from path: {}", icon_path);
            let image = image::open(&icon_path)
                .map_err(|e| format!("Failed to open icon at {}: {}", icon_path, e))?
                .into_rgba8();
            let (width, height) = image.dimensions();
            let rgba = image.into_raw();
            debug!("Icon loaded successfully: {}x{}", width, height);
            (rgba, width, height)
        };
        let icon = tray_icon::Icon::from_rgba(icon_rgba, icon_width, icon_height)
            .expect("Failed to open icon");

        // Create initial menu
        let menu = Self::create_menu(&client);

        let tray_icon = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("Omenix - Fan Control")
            .with_icon(icon)
            .build()
            .expect("Failed to create tray icon");

        info!("System tray icon created successfully");

        Ok(Self {
            tray_icon,
            client,
            cached_state: Arc::new(Mutex::new(None)),
        })
    }

    fn create_menu_with_state(state: &SystemState) -> Menu {
        // Fan control submenu
        let fan_current_mode = state.fan_mode;
        let fan_menu_label = format!("🌪️ Fan Mode: {}", fan_current_mode);

        let fan_max_id = MenuId::new(FAN_MAX_ID);
        let fan_auto_id = MenuId::new(FAN_AUTO_ID);
        let fan_bios_id = MenuId::new(FAN_BIOS_ID);

        let fan_max_label = if fan_current_mode == FanMode::Max {
            "• Max Performance"
        } else {
            "Max Performance"
        };
        let fan_auto_label = if fan_current_mode == FanMode::Auto {
            "• Auto Control"
        } else {
            "Auto Control"
        };
        let fan_bios_label = if fan_current_mode == FanMode::Bios {
            "• BIOS Default"
        } else {
            "BIOS Default"
        };

        let fan_max = MenuItem::with_id(fan_max_id, fan_max_label, true, None);
        let fan_auto = MenuItem::with_id(fan_auto_id, fan_auto_label, true, None);
        let fan_bios = MenuItem::with_id(fan_bios_id, fan_bios_label, true, None);

        let fan_submenu =
            Submenu::with_items(&fan_menu_label, true, &[&fan_max, &fan_auto, &fan_bios])
                .expect("Failed to create fan submenu");

        // Performance mode submenu
        let perf_current_mode = state.performance_mode;
        let perf_menu_label = format!("⚡ Performance: {}", perf_current_mode);

        let perf_balanced_id = MenuId::new(PERF_BALANCED_ID);
        let perf_performance_id = MenuId::new(PERF_PERFORMANCE_ID);

        let perf_balanced_label = if perf_current_mode == PerformanceMode::Balanced {
            "• Balanced"
        } else {
            "Balanced"
        };
        let perf_performance_label = if perf_current_mode == PerformanceMode::Performance {
            "• Performance"
        } else {
            "Performance"
        };

        let perf_balanced = MenuItem::with_id(perf_balanced_id, perf_balanced_label, true, None);
        let perf_performance =
            MenuItem::with_id(perf_performance_id, perf_performance_label, true, None);

        let perf_submenu =
            Submenu::with_items(&perf_menu_label, true, &[&perf_balanced, &perf_performance])
                .expect("Failed to create performance submenu");

        // Quit item
        let quit_id = MenuId::new(QUIT_ID);
        let quit = MenuItem::with_id(quit_id, "Quit", true, None);
        let separator = PredefinedMenuItem::separator();

        Menu::with_items(&[&fan_submenu, &perf_submenu, &separator, &separator, &quit])
            .expect("Failed to create menu")
    }

    fn create_menu(client: &DaemonClient) -> Menu {
        // Get current state to show in menu
        let current_state = client.get_current_state().ok();

        if let Some(state) = current_state {
            Self::create_menu_with_state(&state)
        } else {
            // Fallback for when we can't get state
            Self::create_menu_with_state(&SystemState {
                fan_mode: FanMode::Auto,
                performance_mode: PerformanceMode::Balanced,
                temperature: None,
            })
        }
    }

    fn update_menu(&mut self) {
        // Check if state has actually changed before recreating menu
        if let Ok(current_state) = self.client.get_current_state() {
            let mut cached = self.cached_state.lock().unwrap();
            let should_update = match &*cached {
                Some(old_state) => {
                    // Compare states to see if update is needed
                    old_state.fan_mode != current_state.fan_mode
                        || old_state.performance_mode != current_state.performance_mode
                        || old_state.temperature != current_state.temperature
                }
                None => true, // First time, always update
            };

            if should_update {
                debug!("State changed, updating menu");
                let new_menu = Self::create_menu_with_state(&current_state);
                self.tray_icon.set_menu(Some(Box::new(new_menu)));
                *cached = Some(current_state);
            } else {
                debug!("State unchanged, skipping menu update");
            }
        } else {
            debug!("Failed to get current state, skipping menu update");
        }
    }

    pub fn start_event_loop(
        &mut self,
        tx: mpsc::Sender<TrayMessage>,
        rx_refresh: mpsc::Receiver<()>,
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
                    PERF_BALANCED_ID => {
                        info!("Balanced performance mode clicked");
                        let _ = tx.send(TrayMessage::SetPerformanceMode(PerformanceMode::Balanced));
                    }
                    PERF_PERFORMANCE_ID => {
                        info!("Performance mode clicked");
                        let _ = tx.send(TrayMessage::SetPerformanceMode(
                            PerformanceMode::Performance,
                        ));
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

        // Main GTK event loop with less frequent menu updates
        // Update menu less frequently to reduce flickering
        let mut last_update = std::time::Instant::now();
        let update_interval = std::time::Duration::from_secs(10); // Reduced from 3 to 10 seconds

        loop {
            gtk::main_iteration_do(false); // Don't block

            // Check for refresh signals (non-blocking)
            if let Ok(()) = rx_refresh.try_recv() {
                debug!("Received refresh signal, updating menu immediately");
                self.handle_state_change();
            }

            // Update menu periodically as fallback
            if last_update.elapsed() > update_interval {
                debug!("Periodic menu update");
                self.update_menu();
                last_update = std::time::Instant::now();
            }

            // Longer sleep to reduce CPU usage and potential flickering
            std::thread::sleep(std::time::Duration::from_millis(250));
        }
    }

    pub fn handle_state_change(&mut self) {
        // Force update menu when external state change is detected
        debug!("External state change detected, forcing menu update");
        if let Ok(current_state) = self.client.get_current_state() {
            let new_menu = Self::create_menu_with_state(&current_state);
            self.tray_icon.set_menu(Some(Box::new(new_menu)));
            let mut cached = self.cached_state.lock().unwrap();
            *cached = Some(current_state);
        }
    }
}
