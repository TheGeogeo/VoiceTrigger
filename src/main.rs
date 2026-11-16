#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod journal;
#[cfg(windows)]
mod platform;
mod silence;

fn main() {
    #[cfg(windows)]
    if let Err(error) = platform::run() {
        if std::env::args().any(|argument| argument == "--diagnostic") {
            eprintln!("{error}");
        } else {
            platform::show_error(&error);
        }
        std::process::exit(1);
    }
    #[cfg(not(windows))]
    eprintln!("VoiceTrigger requires Windows 10 or Windows 11.");
}
