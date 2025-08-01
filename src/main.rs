use tray_icon::{menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem}, TrayIconBuilder, TrayIconEvent};

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

    // Handle menu events
    let menu_channel = MenuEvent::receiver();
    let _tray_channel = TrayIconEvent::receiver();

    std::thread::spawn(move || {
        loop {
            if let Ok(event) = menu_channel.recv() {
                match event.id.0.as_str() {
                    "Max Performance" => {
                        println!("Setting fan to max performance");
                        set_fan_mode("max");
                    },
                    "Auto Control" => {
                        println!("Setting fan to auto control");
                        set_fan_mode("auto");
                    },
                    "BIOS Default" => {
                        println!("Setting fan to BIOS default");
                        set_fan_mode("bios");
                    },
                    "Quit" => {
                        println!("Quitting application");
                        gtk::main_quit();
                        break;
                    },
                    _ => {}
                }
            }
        }
    });

    gtk::main();
}

fn set_fan_mode(mode: &str) {
    // TODO: Implement actual fan control logic
    match mode {
        "max" => println!("Fan set to maximum performance"),
        "auto" => println!("Fan set to automatic control"),
        "bios" => println!("Fan set to BIOS default"),
        _ => println!("Unknown fan mode: {}", mode),
    }
}
