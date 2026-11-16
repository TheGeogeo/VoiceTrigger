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
    "Valider la dictée",
    "Confirmer la dictée",
    "Transcrire",
    "Transcribe",
];
const SEND: &[&str] = &[
    "Send prompt",
    "Send message",
    "Send",
    "Envoyer le message",
    "Envoyer",
];
const CANCEL: &[&str] = &[
    "Cancel dictation",
    "Stop dictation",
    "Stop this dictation",
    "Annuler la dictée",
    "Arrêter la dictée",
    "Cancel",
    "Annuler",
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

fn is_chatgpt(path: &std::path::Path) -> bool {
    path.file_name().map_or(false, |name| {
        name.to_string_lossy().eq_ignore_ascii_case("ChatGPT.exe")
    })
}

unsafe extern "system" fn collect_window(window: HWND, data: LPARAM) -> BOOL {
    let mut id = 0;
    GetWindowThreadProcessId(window, Some(&mut id));
    if IsWindowVisible(window).as_bool()
        && GetWindow(window, GW_OWNER).0 == 0
        && process_path(id).map_or(false, |path| is_chatgpt(&path))
    {
        (*(data.0 as *mut Vec<HWND>)).push(window);
    }
    BOOL(1)
}

fn window() -> Result<HWND> {
    let mut windows: Vec<HWND> = Vec::new();
    unsafe {
        EnumWindows(
            Some(collect_window),
            LPARAM(&mut windows as *mut _ as isize),
        )
        .ok()
        .map_err(|e| e.to_string())?;
        let foreground = GetForegroundWindow();
        if windows.contains(&foreground) {
            return Ok(foreground);
        }
    }
    match windows.as_slice() {
        [window] => Ok(*window),
        [] => Err("Aucune fenêtre ChatGPT. Ouvrez l'application ou indiquez son chemin .exe dans les réglages.".into()),
        _ => Err("Plusieurs fenêtres ChatGPT : placez celle à utiliser au premier plan puis réessayez.".into()),
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
    let executable = executable.ok_or(
        "Ouvrez ChatGPT une première fois ou renseignez son chemin .exe dans les réglages.",
    )?;
    std::process::Command::new(executable)
        .spawn()
        .map_err(|e| format!("Ouverture de ChatGPT : {e}"))?;
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
            Err(format!("Impossible d'ouvrir le site (code {}).", result.0))
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
        let mut found = None;
        for button in self.buttons()? {
            let name = unsafe { button.CurrentName() }
                .map_err(|e| e.to_string())?
                .to_string();
            if matches_name(&name, custom, aliases)
                && unsafe { button.CurrentIsEnabled() }.map_or(false, |value| value.as_bool())
            {
                if found.is_some() {
                    return Err(
                        "Plusieurs boutons correspondent. Précisez le libellé dans settings.json."
                            .into(),
                    );
                }
                found = Some(button);
            }
        }
        Ok(found)
    }

    fn invoke(&self, custom: &str, aliases: &[&str], description: &str) -> Result<bool> {
        let Some(button) = self.find(custom, aliases)? else {
            return Ok(false);
        };
        unsafe {
            let pattern: IUIAutomationInvokePattern = button
                .GetCurrentPatternAs(UIA_InvokePatternId)
                .map_err(|e| format!("Bouton {description} non invocable : {e}"))?;
            pattern
                .Invoke()
                .map_err(|e| format!("Bouton {description} : {e}"))?;
        }
        Ok(true)
    }

    pub fn new_chat(&self) -> Result<()> {
        unsafe {
            ShowWindow(self.window, SW_RESTORE);
            SetForegroundWindow(self.window);
        }
        if self.invoke("", NEW_CHAT, "nouvelle conversation")? {
            Ok(())
        } else {
            Err("Bouton Nouvelle conversation introuvable. Passez ChatGPT dans la vue Chat et réessayez.".into())
        }
    }

    pub fn start(&self, settings: &Settings) -> Result<bool> {
        self.invoke(&settings.microphone_button, MICROPHONE, "dictée")
    }

    pub fn finish(&self, settings: &Settings) -> Result<()> {
        if self.invoke(&settings.finish_button, FINISH, "validation de la dictée")? {
            Ok(())
        } else {
            Err("Bouton de validation de la dictée introuvable. Terminez la dictée manuellement dans ChatGPT.".into())
        }
    }

    pub fn send(&self, settings: &Settings) -> Result<bool> {
        self.invoke(&settings.send_button, SEND, "envoi")
    }

    pub fn cancel(&self, settings: &Settings) -> Result<()> {
        if self.invoke(&settings.cancel_button, CANCEL, "annulation")? {
            Ok(())
        } else {
            Err("Annulation automatique indisponible : arrêtez la dictée manuellement dans ChatGPT.".into())
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
    let chat = match ChatGpt::connect() {
        Ok(chat) => chat,
        Err(error) => return Ok(format!("ChatGPT : {error}")),
    };
    let mut report = String::from(
        "ChatGPT : fenêtre détectée. Contrôles disponibles (aucune action exécutée) :",
    );
    for (description, custom, aliases) in [
        ("Nouvelle conversation", "", NEW_CHAT),
        ("Dictée", settings.microphone_button.as_str(), MICROPHONE),
        (
            "Validation de la dictée",
            settings.finish_button.as_str(),
            FINISH,
        ),
        ("Envoi", settings.send_button.as_str(), SEND),
        ("Annulation", settings.cancel_button.as_str(), CANCEL),
    ] {
        let available = chat.find(custom, aliases)?.is_some();
        report.push_str(&format!(
            "\n- {description} : {}",
            if available {
                "oui"
            } else {
                "non (peut dépendre de l'état de la conversation)"
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
        assert!(!matches_name("Send feedback", "", SEND));
        assert!(matches_name(
            "My dictation button",
            "My dictation button",
            MICROPHONE
        ));
        assert!(!matches_name("Dictate", "My dictation button", MICROPHONE));
    }
}
