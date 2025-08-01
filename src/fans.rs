fn set_fan_mode(mode: &str) {
    match mode {
        "max" => println!("Fan set to maximum performance"),
        "auto" => println!("Fan set to automatic control"),
        "bios" => println!("Fan set to BIOS default"),
        _ => println!("Unknown fan mode: {}", mode),
    }
}