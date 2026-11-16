mod audio;
#[cfg(test)]
mod audio_tests;
mod chatgpt;
mod engine;
mod speech;
mod ui;

use windows::core::PCWSTR;
use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_MULTITHREADED};

pub type Result<T> = std::result::Result<T, String>;

pub fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(Some(0)).collect()
}

pub struct Com;

impl Com {
    pub fn new() -> Result<Self> {
        unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) }.map_err(|e| e.to_string())?;
        Ok(Self)
    }
}

impl Drop for Com {
    fn drop(&mut self) {
        unsafe { CoUninitialize() };
    }
}

pub fn run() -> Result<()> {
    let mut arguments = std::env::args().skip(1);
    let mut path = crate::config::default_path();
    let mut diagnostic = false;
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--diagnostic" => diagnostic = true,
            "--config" => {
                path = arguments
                    .next()
                    .ok_or("Missing path after --config")?
                    .into()
            }
            _ => return Err(format!("Unknown option: {argument}")),
        }
    }
    let settings = crate::config::Settings::load(&path)?;
    if diagnostic {
        let _com = Com::new()?;
        let report = format!(
            "Configuration: {}\n\n{}\n\n{}",
            path.display(),
            speech::diagnostic()?,
            chatgpt::diagnostic(&settings)?
        );
        // Release is a GUI executable; also write a report beside the config so diagnostics remain readable.
        let report_path = path.with_extension("diagnostic.txt");
        if let Some(parent) = report_path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::write(&report_path, &report).map_err(|e| e.to_string())?;
        println!("{report}\nReport: {}", report_path.display());
        Ok(())
    } else {
        ui::run(settings, path)
    }
}

pub fn show_error(error: &str) {
    eprintln!("{error}");
    unsafe {
        windows::Win32::UI::WindowsAndMessaging::MessageBoxW(
            None,
            PCWSTR(wide(error).as_ptr()),
            windows::core::w!("VoiceTrigger"),
            windows::Win32::UI::WindowsAndMessaging::MB_OK
                | windows::Win32::UI::WindowsAndMessaging::MB_ICONERROR,
        );
    }
}
