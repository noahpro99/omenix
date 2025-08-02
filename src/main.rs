use std::{sync::{mpsc, Arc, Mutex}, thread};

use gtk::traits::GtkSettingsExt;
use tray_icon::{menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem}, TrayIconBuilder, TrayIconEvent};

mod fans;
use fans::{AppState, FanStatus};

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
    gtk::init().expect("Failed to initialize GTK.");
    let Some(settings) = gtk::Settings::default() else {
        panic!("Failed to get default GTK settings.");
    };
    settings.set_gtk_application_prefer_dark_theme(true);
    let path = "assets/icon.png"; // Replace with your icon path

    // Initialize app state
    let app_state = Arc::new(Mutex::new(AppState::new()));
    
    // Start the fan control background thread
    fans::start_fan_control_thread(app_state.clone());

    let (icon_rgba, icon_width, icon_height) = {
        let image = image::open(path)
            .expect("Failed to open icon path")
            .into_rgba8();
        let (width, height) = image.dimensions();
        let rgba = image.into_raw();
        (rgba, width, height)
    };
    let icon = tray_icon::Icon::from_rgba(icon_rgba, icon_width, icon_height)
        .expect("Failed to open icon");

    let fan_status = MenuItem::new(fans::fan_status_string(app_state.clone()).as_str(), false, None);
    let separator1 = PredefinedMenuItem::separator();
    let fan_max = MenuItem::new("Max Performance", true, None);
    let fan_auto = MenuItem::new("Auto Control", true, None);
    let fan_bios = MenuItem::new("BIOS Default", true, None);
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

    let menu_channel = MenuEvent::receiver();
    let _tray_channel = TrayIconEvent::receiver();


    let (tx, rx) = mpsc::channel();
    let (tx_quit, rx_quit) = mpsc::channel();

    let clone_fan_max_id = fan_max.id().clone();
    let clone_fan_auto_id = fan_auto.id().clone();
    let clone_fan_bios_id = fan_bios.id().clone();
    let quit_id = quit.id().clone();

    let tx_quit_clone = tx_quit.clone();
    thread::spawn(move || {
        while let Ok(event) = menu_channel.recv() {
            if *event.id() == clone_fan_max_id {
                let _ = tx.send(TrayMessage::FansMax);
            } else if *event.id() == clone_fan_auto_id {
                let _ = tx.send(TrayMessage::FansAuto);
            } else if *event.id() == clone_fan_bios_id {
                let _ = tx.send(TrayMessage::FansBios);
            } else if *event.id() == quit_id {
                let _ = tx.send(TrayMessage::Exit);
                let _ = tx_quit_clone.send(());
            }
        }
    });

    // setup handler for rx
    let app_state_for_handler = app_state.clone();
    thread::spawn(move || {
        while let Ok(message) = rx.recv() {
            match message {
                TrayMessage::PerformanceMode => {
                    println!("Switching to Performance Mode...");
                }
                TrayMessage::DefaultMode => {
                    println!("Switching to Default Mode...");
                }
                TrayMessage::FansMax => {
                    println!("Setting fans to Max Performance...");
                    if let Err(e) = fans::set_fan_status(app_state_for_handler.clone(), FanStatus::Max) {
                        eprintln!("Failed to set max fan mode: {}", e);
                    } else {
                        println!("✓ Fan mode set to: Max Performance");
                    }
                }
                TrayMessage::FansAuto => {
                    println!("Setting fans to Auto Control...");
                    if let Err(e) = fans::set_fan_status(app_state_for_handler.clone(), FanStatus::Auto) {
                        eprintln!("Failed to set auto fan mode: {}", e);
                    } else {
                        println!("✓ Fan mode set to: Auto Control (will switch between Max/BIOS based on temperature)");
                    }
                }
                TrayMessage::FansBios => {
                    println!("Setting fans to BIOS Default...");
                    if let Err(e) = fans::set_fan_status(app_state_for_handler.clone(), FanStatus::Bios) {
                        eprintln!("Failed to set bios fan mode: {}", e);
                    } else {
                        println!("✓ Fan mode set to: BIOS Default");
                    }
                }
                TrayMessage::UpdateStatus => {
                    println!("Updating status...");
                }
                TrayMessage::Exit => {
                    println!("Exiting application...");
                    std::process::exit(0);
                }
            }
        }
    });

    // Handle quit signal
    thread::spawn(move || {
        let _ = rx_quit.recv();
        std::process::exit(0);
    });

    gtk::main();
}