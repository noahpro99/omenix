use tray_icon::{TrayIconBuilder, menu::Menu};

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

    let _tray_icon = TrayIconBuilder::new()
        .with_menu(Box::new(Menu::new()))
        .with_tooltip("system-tray - tray icon library!")
        .with_icon(icon)
        .build();

    gtk::main();
}
