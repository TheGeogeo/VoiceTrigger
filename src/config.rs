use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub enum Action {
    ChatGptDictation,
    OpenChatGpt,
    OpenUrl(String),
    Launch(String),
}

impl Action {
    pub fn label(&self) -> &'static str {
        match self {
            Self::ChatGptDictation => "Dictate to ChatGPT",
            Self::OpenChatGpt => "Open ChatGPT",
            Self::OpenUrl(_) => "Open a website",
            Self::Launch(_) => "Launch an application",
        }
    }

    pub fn target(&self) -> &str {
        match self {
            Self::OpenUrl(value) | Self::Launch(value) => value,
            _ => "",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Trigger {
    pub phrase: String,
    pub action: Action,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(default)]
pub struct Settings {
    pub triggers: Vec<Trigger>,
    pub language: u16,
    pub voice_threshold: f32,
    pub chatgpt_path: String,
    pub microphone_button: String,
    pub finish_button: String,
    pub send_button: String,
    pub cancel_button: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            triggers: vec![Trigger {
                phrase: "Dit ChatGPT".into(),
                action: Action::ChatGptDictation,
            }],
            language: 0x040c,
            voice_threshold: 0.015,
            chatgpt_path: String::new(),
            microphone_button: String::new(),
            finish_button: String::new(),
            send_button: String::new(),
            cancel_button: String::new(),
        }
    }
}

pub fn normalize(phrase: &str) -> String {
    phrase
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

impl Settings {
    pub fn validate(&self) -> Result<(), String> {
        if self.triggers.is_empty() || self.triggers.len() > 32 {
            return Err("Add between 1 and 32 triggers.".into());
        }
        if !self.voice_threshold.is_finite() || !(0.001..=0.5).contains(&self.voice_threshold) {
            return Err("The microphone threshold must be between 0.001 and 0.5.".into());
        }
        let mut phrases = std::collections::HashSet::new();
        for trigger in &self.triggers {
            let phrase = normalize(&trigger.phrase);
            if phrase.is_empty()
                || phrase.len() > 160
                || trigger.phrase.chars().any(char::is_control)
            {
                return Err(
                    "Each phrase must contain 1 to 160 bytes, with no control characters.".into(),
                );
            }
            if !phrases.insert(phrase) {
                return Err("Two triggers use the same phrase.".into());
            }
            match &trigger.action {
                Action::OpenUrl(url) => {
                    let lower = url.to_lowercase();
                    if !(lower.starts_with("https://") || lower.starts_with("http://"))
                        || url.chars().any(char::is_whitespace)
                        || url
                            .split_once("://")
                            .map_or(true, |(_, host)| host.is_empty() || host.starts_with('/'))
                    {
                        return Err("Enter a complete HTTP or HTTPS address.".into());
                    }
                }
                Action::Launch(path)
                    if !Path::new(path).is_file() || !path.to_lowercase().ends_with(".exe") =>
                {
                    return Err("Choose an existing .exe file without arguments.".into());
                }
                _ => {}
            }
        }
        if !self.chatgpt_path.is_empty()
            && (!Path::new(&self.chatgpt_path).is_file()
                || !self.chatgpt_path.to_lowercase().ends_with(".exe"))
        {
            return Err("The ChatGPT path must point to an existing .exe file.".into());
        }
        Ok(())
    }

    pub fn load(path: &Path) -> Result<Self, String> {
        let content = match std::fs::read_to_string(path) {
            Ok(content) => content,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self::default())
            }
            Err(error) => return Err(format!("Reading {}: {error}", path.display())),
        };
        let settings: Self = serde_json::from_str(&content)
            .map_err(|error| format!("Invalid JSON configuration: {error}"))?;
        settings.validate()?;
        Ok(settings)
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        self.validate()?;
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let temp = path.with_extension("json.tmp");
        let bytes = serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?;
        {
            use std::io::Write;
            let mut file = std::fs::File::create(&temp).map_err(|e| e.to_string())?;
            file.write_all(&bytes).map_err(|e| e.to_string())?;
            file.sync_all().map_err(|e| e.to_string())?;
        }
        replace_file(&temp, path)
    }
}

#[cfg(windows)]
fn replace_file(source: &Path, target: &Path) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::Storage::FileSystem::{
        MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
    };
    let source: Vec<_> = source.as_os_str().encode_wide().chain(Some(0)).collect();
    let target: Vec<_> = target.as_os_str().encode_wide().chain(Some(0)).collect();
    unsafe {
        MoveFileExW(
            windows::core::PCWSTR(source.as_ptr()),
            windows::core::PCWSTR(target.as_ptr()),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    }
    .ok()
    .map_err(|e| e.to_string())
}

#[cfg(not(windows))]
fn replace_file(source: &Path, target: &Path) -> Result<(), String> {
    std::fs::rename(source, target).map_err(|e| e.to_string())
}

pub fn default_path() -> PathBuf {
    std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("VoiceTrigger")
        .join("settings.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_duplicate_phrases_ignoring_case_and_spaces() {
        let mut settings = Settings::default();
        settings.triggers.push(Trigger {
            phrase: "  DIT   CHATGPT ".into(),
            action: Action::OpenChatGpt,
        });
        assert!(settings.validate().unwrap_err().contains("same phrase"));
    }

    #[test]
    fn rejects_invalid_actions_and_non_finite_threshold() {
        let mut settings = Settings::default();
        settings.triggers[0].action = Action::OpenUrl("file:///C:/Windows".into());
        assert!(settings.validate().is_err());
        settings.triggers[0].action = Action::OpenUrl("https://chatgpt.com".into());
        assert!(settings.validate().is_ok());
        settings.voice_threshold = f32::NAN;
        assert!(settings.validate().is_err());
    }

    #[test]
    fn settings_round_trip_and_replace_existing_file() {
        let path =
            std::env::temp_dir().join(format!("voice-trigger-test-{}.json", std::process::id()));
        let mut settings = Settings::default();
        settings.save(&path).unwrap();
        settings.triggers[0].phrase = "hello computer".into();
        settings.save(&path).unwrap();
        assert_eq!(Settings::load(&path).unwrap(), settings);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn corrupted_configuration_is_not_silently_discarded() {
        let path =
            std::env::temp_dir().join(format!("voice-trigger-invalid-{}.json", std::process::id()));
        std::fs::write(&path, "{invalid").unwrap();
        assert!(Settings::load(&path).unwrap_err().contains("Invalid JSON"));
        std::fs::remove_file(path).unwrap();
    }
}
