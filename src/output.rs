use colored::Colorize;

fn color_enabled() -> bool {
    std::env::var("DPKG_NO_COLOR").is_err() && std::env::var("NO_COLOR").is_err()
}

pub fn success(msg: &str) {
    if color_enabled() {
        println!("{}", msg.green());
    } else {
        println!("{msg}");
    }
}

pub fn error(msg: &str) {
    if color_enabled() {
        eprintln!("{}", msg.red());
    } else {
        eprintln!("{msg}");
    }
}

pub fn warning(msg: &str) {
    if color_enabled() {
        println!("{}", msg.yellow());
    } else {
        println!("{msg}");
    }
}

pub fn info(msg: &str) {
    if color_enabled() {
        println!("{}", msg.blue());
    } else {
        println!("{msg}");
    }
}

pub fn dry_run(msg: &str) {
    if color_enabled() {
        println!("{}", msg.cyan());
    } else {
        println!("{msg}");
    }
}

pub fn plain(msg: &str) {
    println!("{msg}");
}

pub fn added(name: &str, detail: &str) {
    if color_enabled() {
        println!("{} {:<30} {}", "+".green(), name.green(), detail);
    } else {
        println!("+ {name:<30} {detail}");
    }
}

pub fn removed(name: &str, detail: &str) {
    if color_enabled() {
        println!("{} {:<30} {}", "-".red(), name.red(), detail);
    } else {
        println!("- {name:<30} {detail}");
    }
}
