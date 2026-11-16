use super::{wide, Result};
use crate::config::Settings;
use std::path::PathBuf;
use windows::core::{PCWSTR, PWSTR};
use windows::Win32::Foundation::{CloseHandle, BOOL, HWND, LPARAM};
use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_ALL};
use windows::Win32::System::Diagnostics::ToolHelp::*;
use windows::Win32::System::Threading::*;
use windows::Win32::UI::Accessibility::*;
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::*;

// Localized accessibility labels are language data, not application messages.
const NEW_CHAT: &[&str] = &[
    "New chat",
    "Nouveau chat",
    "Nouvelle discussion",
    "Nouvelle conversation",
    "Start new chat",
];
const MICROPHONE: &[&str] = &[
    "Dictate",
    "Start dictation",
    "Use microphone",
    "Dicter",
    "Démarrer la dictée",
    "Utiliser le microphone",
    "Dictée",
];
const FINISH: &[&str] = &[
    "Submit dictation",
    "Confirm dictation",
    "Stop dictation",
    "Stop recording",
    "Arrêter la dictée",
    "Arrêter l'enregistrement",
    "Valider la dictée",
    "Confirmer la dictée",
    "Transcrire",
    "Transcribe",
];
const SEND: &[&str] = &[
    "Send prompt",
    "Send message",
    "Envoyer le prompt",
    "Envoyer le message",
    "Envoyer la demande",
    "Send",
    "Envoyer",
];
const CANCEL: &[&str] = &[
    "Cancel dictation",
    "Annuler la dictée",
    "Stop this dictation",
];

fn process_path(id: u32) -> Option<PathBuf> {
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, id).ok()?;
        let mut buffer = [0u16; 32768];
        let mut length = buffer.len() as u32;
        let result = QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_FORMAT(0),
            PWSTR(buffer.as_mut_ptr()),
            &mut length,
        )
        .as_bool();
        let _ = CloseHandle(handle);
        result.then(|| PathBuf::from(String::from_utf16_lossy(&buffer[..length as usize])))
    }
}

#[derive(Default)]
struct WindowInventory {
    process_ids: Vec<u32>,
    windows: Vec<HWND>,
    owned: usize,
    hidden: usize,
    path_unavailable: usize,
    hidden_main: Vec<HWND>,
}

fn chatgpt_process_ids() -> Result<Vec<u32>> {
    unsafe {
        let snapshot =
            CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0).map_err(|e| e.to_string())?;
        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };
        let mut ids = Vec::new();
        if Process32FirstW(snapshot, &mut entry).as_bool() {
            loop {
                let length = entry
                    .szExeFile
                    .iter()
                    .position(|c| *c == 0)
                    .unwrap_or(entry.szExeFile.len());
                if String::from_utf16_lossy(&entry.szExeFile[..length])
                    .eq_ignore_ascii_case("ChatGPT.exe")
                {
                    ids.push(entry.th32ProcessID);
                }
                if !Process32NextW(snapshot, &mut entry).as_bool() {
                    break;
                }
            }
        }
        let _ = CloseHandle(snapshot);
        Ok(ids)
    }
}

unsafe extern "system" fn collect_window(window: HWND, data: LPARAM) -> BOOL {
    let inventory = &mut *(data.0 as *mut WindowInventory);
    let mut id = 0;
    GetWindowThreadProcessId(window, Some(&mut id));
    if inventory.process_ids.contains(&id) {
        if process_path(id).is_none() {
            inventory.path_unavailable += 1;
        }
        if !IsWindowVisible(window).as_bool() {
            inventory.hidden += 1;
            let mut class = [0u16; 128];
            let length = GetClassNameW(window, &mut class) as usize;
            let browser_window = String::from_utf16_lossy(&class[..length]) == "Chrome_WidgetWin_1";
            let captioned = GetWindowLongPtrW(window, GWL_STYLE) as u32 & WS_CAPTION.0 != 0;
            if GetWindow(window, GW_OWNER).0 == 0
                && GetWindowTextLengthW(window) > 0
                && (browser_window || captioned)
            {
                inventory.hidden_main.push(window);
            }
        } else {
            if GetWindow(window, GW_OWNER).0 != 0 {
                inventory.owned += 1;
            }
            inventory.windows.push(window);
        }
    }
    BOOL(1)
}

fn window_inventory() -> Result<WindowInventory> {
    let mut inventory = WindowInventory {
        process_ids: chatgpt_process_ids()?,
        ..Default::default()
    };
    unsafe {
        EnumWindows(
            Some(collect_window),
            LPARAM(&mut inventory as *mut _ as isize),
        )
        .ok()
        .map_err(|e| e.to_string())?;
    }
    Ok(inventory)
}

fn window() -> Result<HWND> {
    let inventory = window_inventory()?;
    let windows = inventory.windows;
    let foreground = unsafe { GetForegroundWindow() };
    if windows.contains(&foreground) {
        return Ok(foreground);
    }
    let roots: Vec<_> = windows
        .iter()
        .copied()
        .filter(|window| unsafe { GetWindow(*window, GW_OWNER).0 == 0 })
        .collect();
    let candidates = if roots.is_empty() { &windows } else { &roots };
    match candidates.as_slice() {
        [window] => Ok(*window),
        [] => Err("No ChatGPT window is available. Open the app or enter its .exe path in settings.".into()),
        _ => Err("Multiple ChatGPT windows are open. Bring the intended window to the foreground and try again.".into()),
    }
}

fn running_executable() -> Option<PathBuf> {
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0).ok()?;
        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };
        let mut found = None;
        if Process32FirstW(snapshot, &mut entry).as_bool() {
            loop {
                let length = entry
                    .szExeFile
                    .iter()
                    .position(|c| *c == 0)
                    .unwrap_or(entry.szExeFile.len());
                if String::from_utf16_lossy(&entry.szExeFile[..length])
                    .eq_ignore_ascii_case("ChatGPT.exe")
                {
                    found = process_path(entry.th32ProcessID);
                    if found.is_some() {
                        break;
                    }
                }
                if !Process32NextW(snapshot, &mut entry).as_bool() {
                    break;
                }
            }
        }
        let _ = CloseHandle(snapshot);
        found
    }
}

pub fn open(settings: &Settings) -> Result<()> {
    if let Ok(window) = window() {
        unsafe {
            ShowWindow(window, SW_RESTORE);
            SetForegroundWindow(window);
        }
        return Ok(());
    }
    let inventory = window_inventory()?;
    let primary: Vec<_> = inventory
        .hidden_main
        .iter()
        .copied()
        .filter(|window| unsafe {
            let mut title = [0u16; 128];
            let length = GetWindowTextW(*window, &mut title) as usize;
            String::from_utf16_lossy(&title[..length]).eq_ignore_ascii_case("ChatGPT")
        })
        .collect();
    let restorable = if primary.len() == 1 {
        primary.as_slice()
    } else {
        inventory.hidden_main.as_slice()
    };
    if let [main] = restorable {
        unsafe {
            ShowWindow(*main, SW_RESTORE);
            SetForegroundWindow(*main);
        }
        return Ok(());
    }
    let executable = if settings.chatgpt_path.is_empty() {
        running_executable().or_else(|| {
            std::env::var_os("LOCALAPPDATA")
                .map(PathBuf::from)
                .map(|path| path.join("Programs/ChatGPT/ChatGPT.exe"))
                .filter(|path| path.is_file())
        })
    } else {
        Some(PathBuf::from(&settings.chatgpt_path))
    };
    let executable = executable.ok_or("Open ChatGPT once or enter its .exe path in settings.")?;
    std::process::Command::new(executable)
        .spawn()
        .map_err(|e| format!("Opening ChatGPT: {e}"))?;
    Ok(())
}

pub fn open_url(url: &str) -> Result<()> {
    unsafe {
        let result = ShellExecuteW(
            None,
            windows::core::w!("open"),
            PCWSTR(wide(url).as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        );
        if result.0 <= 32 {
            Err(format!("Could not open the website (code {}).", result.0))
        } else {
            Ok(())
        }
    }
}

pub struct ChatGpt {
    automation: IUIAutomation,
    root: IUIAutomationElement,
    window: HWND,
}

impl ChatGpt {
    pub fn connect() -> Result<Self> {
        unsafe {
            let window = window()?;
            let automation: IUIAutomation =
                CoCreateInstance(&CUIAutomation, None, CLSCTX_ALL).map_err(|e| e.to_string())?;
            let root = automation
                .ElementFromHandle(window)
                .map_err(|e| e.to_string())?;
            Ok(Self {
                automation,
                root,
                window,
            })
        }
    }

    fn buttons(&self) -> Result<Vec<IUIAutomationElement>> {
        unsafe {
            let mut value = windows::Win32::System::Com::VARIANT::default();
            let inner = &mut *value.Anonymous.Anonymous;
            inner.vt = windows::Win32::System::Com::VT_I4;
            inner.Anonymous.lVal = UIA_ButtonControlTypeId.0 as i32;
            let condition = self
                .automation
                .CreatePropertyCondition(UIA_ControlTypePropertyId, value)
                .map_err(|e| e.to_string())?;
            let elements = self
                .root
                .FindAll(TreeScope_Descendants, &condition)
                .map_err(|e| e.to_string())?;
            let count = elements.Length().map_err(|e| e.to_string())?;
            let mut buttons = Vec::new();
            for index in 0..count {
                let element = elements.GetElement(index).map_err(|e| e.to_string())?;
                if element.CurrentControlType().ok() == Some(UIA_ButtonControlTypeId)
                    && !element
                        .CurrentIsOffscreen()
                        .map_or(true, |value| value.as_bool())
                {
                    buttons.push(element);
                }
            }
            Ok(buttons)
        }
    }

    fn find(&self, custom: &str, aliases: &[&str]) -> Result<Option<IUIAutomationElement>> {
        let mut matches = Vec::new();
        for button in self.buttons()? {
            let name = unsafe { button.CurrentName() }
                .map_err(|e| e.to_string())?
                .to_string();
            if matches_name(&name, custom, aliases)
                && unsafe { button.CurrentIsEnabled() }.map_or(false, |value| value.as_bool())
            {
                let priority = if custom.is_empty() {
                    aliases
                        .iter()
                        .position(|alias| name.trim().to_lowercase() == alias.to_lowercase())
                        .unwrap()
                } else {
                    0
                };
                matches.push((priority, button, name));
            }
        }
        let Some(priority) = matches.iter().map(|(priority, _, _)| *priority).min() else {
            return Ok(None);
        };
        let mut preferred = matches
            .into_iter()
            .filter(|(candidate, _, _)| *candidate == priority);
        let (_, button, name) = preferred.next().unwrap();
        if preferred.next().is_some() {
            return Err(format!("Multiple visible buttons have the same label '{name}'. Set a more specific label in settings.json."));
        }
        Ok(Some(button))
    }

    fn invoke(&self, custom: &str, aliases: &[&str], description: &str) -> Result<bool> {
        let Some(button) = self.find(custom, aliases)? else {
            return Ok(false);
        };
        unsafe {
            let pattern: IUIAutomationInvokePattern = button
                .GetCurrentPatternAs(UIA_InvokePatternId)
                .map_err(|e| format!("The {description} button cannot be invoked: {e}"))?;
            pattern
                .Invoke()
                .map_err(|e| format!("The {description} button: {e}"))?;
        }
        Ok(true)
    }

    pub fn new_chat(&self) -> Result<bool> {
        unsafe {
            ShowWindow(self.window, SW_RESTORE);
            SetForegroundWindow(self.window);
        }
        self.invoke("", NEW_CHAT, "new chat")
    }

    pub fn start(&self, settings: &Settings) -> Result<bool> {
        self.invoke(&settings.microphone_button, MICROPHONE, "dictation")
    }

    pub fn recording_ready(&self, settings: &Settings) -> Result<bool> {
        Ok(self.find(&settings.finish_button, FINISH)?.is_some()
            && self.find(&settings.cancel_button, CANCEL)?.is_some())
    }

    pub fn cancel_if_present(&self, settings: &Settings) -> Result<bool> {
        self.invoke(&settings.cancel_button, CANCEL, "dictation cancellation")
    }

    pub fn finish(&self, settings: &Settings) -> Result<()> {
        if self.invoke(&settings.finish_button, FINISH, "dictation confirmation")? {
            Ok(())
        } else {
            Err(
                "Dictation confirmation button not found. Finish dictation manually in ChatGPT."
                    .into(),
            )
        }
    }

    pub fn send(&self, settings: &Settings) -> Result<bool> {
        self.invoke(&settings.send_button, SEND, "send")
    }

    pub fn cancel(&self, settings: &Settings) -> Result<()> {
        if self.invoke(&settings.cancel_button, CANCEL, "cancellation")? {
            Ok(())
        } else {
            Err(
                "Automatic cancellation is unavailable. Cancel dictation manually in ChatGPT."
                    .into(),
            )
        }
    }
}

fn matches_name(name: &str, custom: &str, aliases: &[&str]) -> bool {
    let name = name.trim();
    if !custom.is_empty() {
        return name.eq_ignore_ascii_case(custom);
    }
    aliases
        .iter()
        .any(|alias| name.to_lowercase() == alias.to_lowercase())
}

pub fn diagnostic(settings: &Settings) -> Result<String> {
    let inventory = window_inventory()?;
    let discovery = format!("Window discovery: {} ChatGPT processes, {} visible windows, {} hidden windows ({} restorable main windows), {} owned windows, {} unavailable process paths.",
        inventory.process_ids.len(), inventory.windows.len(), inventory.hidden, inventory.hidden_main.len(), inventory.owned, inventory.path_unavailable);
    let chat = match ChatGpt::connect() {
        Ok(chat) => chat,
        Err(error) => return Ok(format!("{discovery}\nChatGPT: {error}")),
    };
    let mut report =
        format!("{discovery}\nChatGPT window found. Available controls (no action performed):");
    for (description, custom, aliases) in [
        ("New conversation", "", NEW_CHAT),
        ("Dictation", settings.microphone_button.as_str(), MICROPHONE),
        (
            "Dictation confirmation",
            settings.finish_button.as_str(),
            FINISH,
        ),
        ("Send", settings.send_button.as_str(), SEND),
        ("Cancellation", settings.cancel_button.as_str(), CANCEL),
    ] {
        let available = chat.find(custom, aliases)?.is_some();
        report.push_str(&format!(
            "\n- {description} : {}",
            if available {
                "yes"
            } else {
                "no (may depend on the current conversation state)"
            }
        ));
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selects_only_exact_dictation_controls() {
        assert!(matches_name("Dicter", "", MICROPHONE));
        assert!(!matches_name("Start voice mode", "", MICROPHONE));
        assert!(!matches_name("Stop this dictation", "", FINISH));
        assert!(matches_name("Arrêter la dictée", "", FINISH));
        assert!(!matches_name("Arrêter la dictée", "", CANCEL));
        assert!(!matches_name("Annuler", "", CANCEL));
        assert!(!matches_name("Send feedback", "", SEND));
        assert!(matches_name(
            "My dictation button",
            "My dictation button",
            MICROPHONE
        ));
        assert!(!matches_name("Dictate", "My dictation button", MICROPHONE));
    }

    #[test]
    #[ignore = "Opens a new ChatGPT chat, briefly starts dictation and cancels it; never sends"]
    fn native_dictation_starts_and_can_be_cancelled() {
        let _com = crate::platform::Com::new().unwrap();
        let settings = Settings::default();
        open(&settings).unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        let chat = loop {
            match ChatGpt::connect() {
                Ok(chat) => break chat,
                Err(error) if std::time::Instant::now() >= deadline => panic!("{error}"),
                Err(_) => std::thread::sleep(std::time::Duration::from_millis(100)),
            }
        };
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while chat.find("", NEW_CHAT).unwrap().is_none() {
            assert!(
                std::time::Instant::now() < deadline,
                "New chat control must become ready"
            );
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        chat.new_chat().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(700));
        let mut monitor = crate::platform::audio::Microphone::open().unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while !chat.start(&settings).unwrap() {
            assert!(
                std::time::Instant::now() < deadline,
                "Dictation control must become ready"
            );
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        let mut names = Vec::new();
        let mut ready = false;
        while std::time::Instant::now() < deadline {
            names = chat
                .buttons()
                .unwrap()
                .iter()
                .filter_map(|button| unsafe { button.CurrentName().ok() })
                .map(|name| name.to_string())
                .filter(|name| {
                    matches_name(name, "", CANCEL)
                        || matches_name(name, "", FINISH)
                        || matches_name(name, "", SEND)
                })
                .collect();
            if chat
                .find(&settings.cancel_button, CANCEL)
                .unwrap()
                .is_some()
            {
                ready = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        let finish_available = chat
            .find(&settings.finish_button, FINISH)
            .unwrap()
            .is_some();
        let monitor_result = (|| -> Result<()> {
            monitor.start()?;
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(1);
            while std::time::Instant::now() < deadline {
                if monitor.next_level()?.is_some() {
                    return Ok(());
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            Err("No silence-monitor buffer arrived while ChatGPT was recording.".into())
        })();
        drop(monitor);
        let cancellation = chat.cancel(&settings);
        assert!(ready, "Recording cancellation must be available; controls: {names:?}; cancellation: {cancellation:?}");
        cancellation.unwrap();
        assert!(
            finish_available,
            "Recording confirmation must be available; controls: {names:?}"
        );
        monitor_result.unwrap();
    }

    #[test]
    #[ignore = "Restores the installed ChatGPT window and reads controls; does not record or send"]
    fn restored_chatgpt_window_exposes_controls() {
        let _com = crate::platform::Com::new().unwrap();
        let settings = Settings::default();
        open(&settings).unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        loop {
            match ChatGpt::connect() {
                Ok(chat) => {
                    println!("{}", diagnostic(&settings).unwrap());
                    assert!(!chat.buttons().unwrap().is_empty());
                    return;
                }
                Err(error) if std::time::Instant::now() >= deadline => {
                    panic!("{error}; {}", diagnostic(&settings).unwrap())
                }
                Err(_) => std::thread::sleep(std::time::Duration::from_millis(200)),
            }
        }
    }
}
