use super::{audio::Microphone, chatgpt, speech::Listener, Com, Result};
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
    thread: Option<JoinHandle<()>>,
}

impl Worker {
    pub fn start(settings: Settings, events: Events) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let cancellation = stop.clone();
        let thread = thread::spawn(move || {
            if let Err(error) = listen(&settings, &cancellation, &events) {
                let _ = events.send(Event::Status(format!("Erreur : {error}")));
            }
            let _ = events.send(Event::Stopped);
        });
        Self {
            stop,
            thread: Some(thread),
        }
    }

    pub fn stop(&self) {
        self.stop.store(true, Ordering::Relaxed);
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

fn listen(settings: &Settings, stop: &AtomicBool, events: &Events) -> Result<()> {
    settings.validate()?;
    let _com = Com::new()?;
    let mut listener = Some(Listener::new(settings)?);
    status(
        events,
        "Écoute active. Vous pouvez réduire VoiceTrigger dans la zone de notification.",
    );
    while !stop.load(Ordering::Relaxed) {
        let Some(index) = listener.as_ref().unwrap().poll()? else {
            continue;
        };
        let trigger = settings
            .triggers
            .get(index)
            .ok_or("Identifiant de déclencheur inconnu")?;
        status(events, format!("Détecté : « {} »", trigger.phrase));
        // Release the recognition microphone before ChatGPT opens its own recording.
        drop(listener.take());
        let result = match &trigger.action {
            Action::ChatGptDictation => dictate(settings, stop, events),
            Action::OpenChatGpt => chatgpt::open(settings),
            Action::OpenUrl(url) => chatgpt::open_url(url),
            Action::Launch(path) => std::process::Command::new(path)
                .spawn()
                .map(|_| ())
                .map_err(|e| e.to_string()),
        };
        match result {
            Ok(()) => status(events, "Action terminée. Réactivation de l'écoute…"),
            Err(error) => {
                // Fail closed: do not trigger again while ChatGPT might still be recording.
                return Err(error);
            }
        }
        if !pause(stop, Duration::from_secs(2)) {
            break;
        }
        listener = Some(Listener::new(settings)?);
        status(events, "Écoute active — en attente d'un déclencheur.");
    }
    status(events, "Écoute arrêtée.");
    Ok(())
}

fn dictate(settings: &Settings, stop: &AtomicBool, events: &Events) -> Result<()> {
    chatgpt::open(settings)?;
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
    chat.new_chat()?;
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
            return Err("Bouton de dictée introuvable. Ouvrez une conversation ChatGPT, vérifiez les libellés avec Diagnostic, puis réessayez.".into());
        }
        pause(stop, Duration::from_millis(250));
    }
    let result = record(&mut microphone, settings, stop, events);
    drop(microphone);
    match result {
        Ok(true) => {}
        Ok(false) => {
            chat.cancel(settings)?;
            status(events, "Dictée annulée sans envoi.");
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
    status(events, "3 secondes de silence. Transcription ChatGPT…");
    chat.finish(settings)?;
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        // On cancellation leave the text as a draft; never send it.
        if stop.load(Ordering::Relaxed) {
            return Ok(());
        }
        if chat.send(settings)? {
            status(events, "Message envoyé à ChatGPT.");
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err("Envoi indisponible après 30 s. Le texte reste dans ChatGPT ; vérifiez la transcription et envoyez manuellement.".into());
        }
        pause(stop, Duration::from_millis(250));
    }
}

fn record(
    microphone: &mut Microphone,
    settings: &Settings,
    stop: &AtomicBool,
    events: &Events,
) -> Result<bool> {
    // Keep the readiness tone out of the level detector.
    unsafe {
        windows::Win32::System::Diagnostics::Debug::Beep(880, 80);
    }
    microphone.start()?;
    status(
        events,
        "Parlez maintenant. Envoi après 3 secondes de silence. Arrêter annule la dictée.",
    );
    let mut detector = SilenceDetector::new();
    let mut captured = Duration::ZERO;
    let start = Instant::now();
    let mut last_buffer = Instant::now();
    loop {
        if stop.load(Ordering::Relaxed) {
            return Ok(false);
        }
        if start.elapsed() >= Duration::from_secs(120) {
            return Ok(false);
        }
        if last_buffer.elapsed() > Duration::from_secs(2) {
            return Err("Le microphone ne fournit plus de données. Dictée interrompue.".into());
        }
        while let Some((level, chunk)) = microphone.next_level()? {
            last_buffer = Instant::now();
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
