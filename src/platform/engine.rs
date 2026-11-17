use super::{
    audio::Microphone,
    chatgpt,
    speech::{Listener, SpeechEvent},
    Com, Result,
};
use crate::config::{Action, Settings};
use crate::silence::{Outcome, SilenceDetector};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::Sender,
    Arc,
};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

pub enum Event {
    Status(String),
    Stopped,
    DiagnosticDone,
    DiagnosticReport(String),
    Trace(String),
    AudioLevel(u8),
}

#[derive(Clone)]
pub struct Events {
    pub sender: Sender<Event>,
    pub window: isize,
}

impl Events {
    pub fn send(&self, event: Event) -> std::result::Result<(), std::sync::mpsc::SendError<Event>> {
        self.sender.send(event)?;
        unsafe {
            windows::Win32::UI::WindowsAndMessaging::PostMessageW(
                windows::Win32::Foundation::HWND(self.window),
                super::ui::EVENT_MESSAGE,
                windows::Win32::Foundation::WPARAM(0),
                windows::Win32::Foundation::LPARAM(0),
            );
        }
        Ok(())
    }
}

pub struct Worker {
    stop: Arc<AtomicBool>,
    debug: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl Worker {
    pub fn start(settings: Settings, events: Events, debug_enabled: bool) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let debug = Arc::new(AtomicBool::new(debug_enabled));
        let cancellation = stop.clone();
        let diagnostics = debug.clone();
        let thread = thread::spawn(move || {
            if let Err(error) = listen(&settings, &cancellation, &diagnostics, &events) {
                let _ = events.send(Event::Status(format!("Error: {error}")));
            }
            let _ = events.send(Event::Stopped);
        });
        Self {
            stop,
            debug,
            thread: Some(thread),
        }
    }

    pub fn stop(&self) {
        self.stop.store(true, Ordering::Relaxed);
    }

    pub fn set_debug(&self, enabled: bool) {
        self.debug.store(enabled, Ordering::Relaxed);
    }

    pub fn is_finished(&self) -> bool {
        self.thread
            .as_ref()
            .map_or(true, |thread| thread.is_finished())
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        self.stop();
        // Never block the UI on an unresponsive third-party accessibility provider.
        if self.is_finished() {
            if let Some(thread) = self.thread.take() {
                let _ = thread.join();
            }
        }
    }
}

fn status(events: &Events, text: impl Into<String>) {
    let _ = events.send(Event::Status(text.into()));
}

fn pause(stop: &AtomicBool, duration: Duration) -> bool {
    let start = Instant::now();
    while start.elapsed() < duration {
        if stop.load(Ordering::Relaxed) {
            return false;
        }
        thread::sleep(Duration::from_millis(50));
    }
    !stop.load(Ordering::Relaxed)
}

fn listen(
    settings: &Settings,
    stop: &AtomicBool,
    debug: &AtomicBool,
    events: &Events,
) -> Result<()> {
    settings.validate()?;
    let _com = Com::new()?;
    let mut listener = Some(Listener::new(settings)?);
    let _ = events.send(Event::Trace(format!(
        "Input: {}",
        listener.as_ref().unwrap().input_name()?
    )));
    let _ = events.send(Event::Trace(format!(
        "Watching {} configured trigger(s).",
        settings.triggers.len()
    )));
    for trigger in &settings.triggers {
        let _ = events.send(Event::Trace(format!(
            "Armed trigger: '{}' -> {}",
            trigger.phrase,
            trigger.action.label()
        )));
    }
    let mut last_level = Instant::now() - Duration::from_secs(1);
    status(
        events,
        "Listening. Say your trigger phrase. You can hide VoiceTrigger in the notification area.",
    );
    while !stop.load(Ordering::Relaxed) {
        let active_debug = debug.load(Ordering::Relaxed);
        listener.as_mut().unwrap().set_debug(active_debug)?;
        let mut matched = None;
        let mut level = None;
        for event in listener.as_ref().unwrap().poll()? {
            match event {
                SpeechEvent::Recognized {
                    index,
                    text,
                    confidence,
                } => {
                    let confidence_name = if confidence > 0 { "high" } else { "normal" };
                    let _ = events.send(Event::Trace(format!(
                        "Recognized: {text} (confidence: {confidence_name}; SAPI value {confidence})."
                    )));
                    matched.get_or_insert(index);
                }
                SpeechEvent::Trace(message) => {
                    let _ = events.send(Event::Trace(message));
                }
                SpeechEvent::AudioLevel(value) => level = Some(value),
            }
        }
        if active_debug && last_level.elapsed() >= Duration::from_millis(200) {
            if let Some(value) = level {
                let _ = events.send(Event::AudioLevel(value));
                last_level = Instant::now();
            }
        }
        let Some(index) = matched else {
            continue;
        };
        let trigger = settings
            .triggers
            .get(index)
            .ok_or("Unknown trigger identifier")?;
        status(events, format!("Trigger detected: '{}'", trigger.phrase));
        let _ = events.send(Event::Trace(format!(
            "Running action: {}",
            trigger.action.label()
        )));
        // Release the recognition microphone before ChatGPT opens its own recording.
        drop(listener.take());
        let result = match &trigger.action {
            Action::ChatGptDictation => dictate(settings, stop, debug, events),
            Action::OpenChatGpt => chatgpt::open(settings),
            Action::OpenUrl(url) => chatgpt::open_url(url),
            Action::Launch(path) => std::process::Command::new(path)
                .spawn()
                .map(|_| ())
                .map_err(|e| e.to_string()),
        };
        match result {
            Ok(()) => status(events, "Action completed. Resuming trigger listening..."),
            Err(error) => {
                // Fail closed: do not trigger again while ChatGPT might still be recording.
                return Err(error);
            }
        }
        if !pause(stop, Duration::from_secs(2)) {
            break;
        }
        listener = Some(Listener::new(settings)?);
        status(events, "Listening for a configured trigger phrase.");
    }
    status(events, "Listening stopped.");
    Ok(())
}

fn dictate(
    settings: &Settings,
    stop: &AtomicBool,
    debug: &AtomicBool,
    events: &Events,
) -> Result<()> {
    status(events, "Opening or restoring ChatGPT...");
    chatgpt::open(settings)?;
    status(events, "Waiting for the ChatGPT window...");
    let deadline = Instant::now() + Duration::from_secs(10);
    let chat = loop {
        if stop.load(Ordering::Relaxed) {
            return Ok(());
        }
        match chatgpt::ChatGpt::connect() {
            Ok(chat) => break chat,
            Err(error) if Instant::now() >= deadline => return Err(error),
            Err(_) => {
                pause(stop, Duration::from_millis(250));
            }
        }
    };
    status(events, "Opening a new ChatGPT conversation...");
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        if stop.load(Ordering::Relaxed) {
            return Ok(());
        }
        if chat.new_chat()? {
            break;
        }
        if Instant::now() >= deadline {
            return Err(
                "New chat button did not become ready. Select the Chat view in ChatGPT and retry."
                    .into(),
            );
        }
        pause(stop, Duration::from_millis(200));
    }
    // The old composer's Dictate button can remain accessible briefly
    // while ChatGPT asynchronously navigates to a new conversation.
    status(events, "Waiting for the new conversation to settle...");
    if !pause(stop, Duration::from_millis(700)) {
        return Ok(());
    }
    status(events, "Starting ChatGPT dictation...");
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut microphone = Microphone::open()?;
    loop {
        if stop.load(Ordering::Relaxed) {
            return Ok(());
        }
        if chat.start(settings)? {
            break;
        }
        if Instant::now() >= deadline {
            return Err("Dictation button not found. Open a ChatGPT chat, check the accessible labels with Diagnostics, and try again.".into());
        }
        pause(stop, Duration::from_millis(250));
    }
    status(events, "Waiting for ChatGPT recording controls...");
    let abort_recording = |error: String| match chat.cancel_if_present(settings) {
        Ok(true) => format!("{error} Dictation was cancelled."),
        Ok(false) => error,
        Err(cancel_error) => {
            format!("{error} {cancel_error} Cancel dictation manually in ChatGPT.")
        }
    };
    let deadline = Instant::now() + Duration::from_secs(5);
    while !chat.recording_ready(settings).map_err(abort_recording)? {
        if stop.load(Ordering::Relaxed) {
            chat.cancel_if_present(settings)?;
            return Ok(());
        }
        if Instant::now() >= deadline {
            chat.cancel_if_present(settings)?;
            return Err("ChatGPT did not enter recording mode. Check its microphone permission and dictation controls, then retry.".into());
        }
        pause(stop, Duration::from_millis(100));
    }
    let _ = events.send(Event::Trace(
        "ChatGPT recording confirmed: Stop dictation and Cancel dictation are available.".into(),
    ));
    let result = record(&mut microphone, settings, stop, debug, events);
    drop(microphone);
    match result {
        Ok(true) => {}
        Ok(false) => {
            chat.cancel(settings)?;
            status(events, "Dictation cancelled without sending.");
            return Ok(());
        }
        Err(error) => {
            return match chat.cancel(settings) {
                Ok(()) => Err(error),
                Err(cancel) => Err(format!("{error} {cancel}")),
            };
        }
    }
    if stop.load(Ordering::Relaxed) {
        chat.cancel(settings)?;
        return Ok(());
    }
    status(
        events,
        "3 seconds of silence. Waiting for ChatGPT transcription...",
    );
    chat.finish(settings).map_err(abort_recording)?;
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        // On cancellation leave the text as a draft; never send it.
        if stop.load(Ordering::Relaxed) {
            return Ok(());
        }
        if chat.send(settings)? {
            status(events, "Message sent to ChatGPT.");
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err("Send is unavailable after 30 seconds. Check the draft in ChatGPT and send it manually.".into());
        }
        pause(stop, Duration::from_millis(250));
    }
}

fn record(
    microphone: &mut Microphone,
    settings: &Settings,
    stop: &AtomicBool,
    debug: &AtomicBool,
    events: &Events,
) -> Result<bool> {
    // Keep the readiness tone out of the level detector.
    unsafe {
        windows::Win32::System::Diagnostics::Debug::Beep(880, 80);
    }
    microphone.start()?;
    status(
        events,
        "Speak now. The message will be sent after 3 seconds of silence. Stop cancels dictation.",
    );
    let mut detector = SilenceDetector::new();
    let mut captured = Duration::ZERO;
    let start = Instant::now();
    let mut last_buffer = Instant::now();
    let mut last_level = Instant::now() - Duration::from_secs(1);
    loop {
        if stop.load(Ordering::Relaxed) {
            return Ok(false);
        }
        if start.elapsed() >= Duration::from_secs(120) {
            return Ok(false);
        }
        if last_buffer.elapsed() > Duration::from_secs(2) {
            return Err("The microphone stopped delivering audio. Dictation interrupted.".into());
        }
        while let Some((level, chunk)) = microphone.next_level()? {
            last_buffer = Instant::now();
            if debug.load(Ordering::Relaxed) && last_level.elapsed() >= Duration::from_millis(200) {
                let _ = events.send(Event::AudioLevel(
                    (level * 100.0).round().clamp(0.0, 100.0) as u8
                ));
                last_level = Instant::now();
            }
            captured += chunk;
            match detector.update(captured, chunk, level >= settings.voice_threshold) {
                Outcome::Submit => return Ok(true),
                Outcome::Cancel => return Ok(false),
                Outcome::Listening => {}
            }
        }
        thread::sleep(Duration::from_millis(20));
    }
}
