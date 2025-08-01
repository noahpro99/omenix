use std::{sync::mpsc, thread};

use tray_icon::{menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem}, TrayIconBuilder, TrayIconEvent};

mod fans;

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
    let path = "assets/icon.png"; // Replace with your icon path

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

    // Create menu items
    let fan_status = MenuItem::new("Fan State: Auto", false, None);
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

    let clone_fan_max_id = fan_max.id().clone();
    let clone_fan_auto_id = fan_auto.id().clone();
    let clone_fan_bios_id = fan_bios.id().clone();
    let quit_id = quit.id().clone();

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
                gtk::main_quit();
            }
        }
    });

    // setup handler for rx
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
                }
                TrayMessage::FansAuto => {
                    println!("Setting fans to Auto Control...");
                }
                TrayMessage::FansBios => {
                    println!("Setting fans to BIOS Default...");
                }
                TrayMessage::UpdateStatus => {
                    println!("Updating status...");
                }
                TrayMessage::Exit => {
                    gtk::main_quit();
                }
            }
        }
    });

    gtk::main();
}