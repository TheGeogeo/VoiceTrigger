use super::{
    engine::{Event, Events, Worker},
    wide, Result,
};
use crate::config::{Action, Settings, Trigger};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
use windows::Win32::UI::Shell::*;
use windows::Win32::UI::WindowsAndMessaging::*;

pub const EVENT_MESSAGE: u32 = WM_APP + 1;
const TRAY_MESSAGE: u32 = WM_APP + 2;
const SHOW_MESSAGE: u32 = WM_APP + 3;
const QUIT_MESSAGE: u32 = WM_APP + 4;
const LIST: u16 = 101;
const PHRASE: u16 = 102;
const ACTION: u16 = 103;
const TARGET: u16 = 104;
const APPLY: u16 = 105;
const ADD: u16 = 106;
const DELETE: u16 = 107;
const SAVE: u16 = 108;
const START: u16 = 109;
const STOP: u16 = 110;
const DIAGNOSTIC: u16 = 111;
const LANGUAGE: u16 = 112;
const THRESHOLD: u16 = 113;
const CHAT_PATH: u16 = 114;
const STATUS: u16 = 115;
const HIDE: u16 = 116;
const OPEN: u16 = 117;
const EXIT: u16 = 118;

struct App {
    window: HWND,
    settings: Settings,
    path: PathBuf,
    font: HFONT,
    heading_font: HFONT,
    worker: Option<Worker>,
    events: Events,
    receiver: Receiver<Event>,
    diagnostic_active: bool,
    tray: bool,
    exit_when_stopped: bool,
}

impl App {
    unsafe fn control(
        &self,
        class: PCWSTR,
        text: &str,
        style: WINDOW_STYLE,
        id: u16,
        rect: (i32, i32, i32, i32),
    ) -> HWND {
        let (x, y, width, height) = rect;
        let window = CreateWindowExW(
            WINDOW_EX_STYLE(0),
            class,
            PCWSTR(wide(text).as_ptr()),
            WS_CHILD | WS_VISIBLE | style,
            x,
            y,
            width,
            height,
            self.window,
            HMENU(id as isize),
            None,
            None,
        );
        SendMessageW(window, WM_SETFONT, WPARAM(self.font.0 as usize), LPARAM(1));
        window
    }

    unsafe fn label(&self, text: &str, rect: (i32, i32, i32, i32)) -> HWND {
        self.control(w!("STATIC"), text, WINDOW_STYLE(0), 0, rect)
    }

    unsafe fn edit(&self, text: &str, id: u16, rect: (i32, i32, i32, i32)) {
        self.control(
            w!("EDIT"),
            text,
            WS_BORDER | WS_TABSTOP | WINDOW_STYLE(ES_AUTOHSCROLL as u32),
            id,
            rect,
        );
    }

    unsafe fn button(&self, text: &str, id: u16, rect: (i32, i32, i32, i32)) {
        self.control(w!("BUTTON"), text, WS_TABSTOP, id, rect);
    }

    unsafe fn child(&self, id: u16) -> HWND {
        GetDlgItem(self.window, id as i32)
    }

    unsafe fn text(&self, id: u16) -> String {
        let child = self.child(id);
        let mut text = vec![0u16; GetWindowTextLengthW(child) as usize + 1];
        let count = GetWindowTextW(child, &mut text);
        String::from_utf16_lossy(&text[..count as usize])
    }

    unsafe fn set_text(&self, id: u16, text: &str) {
        SetWindowTextW(self.child(id), PCWSTR(wide(text).as_ptr()));
    }

    unsafe fn initialize(&mut self) {
        let heading = self.label("VoiceTrigger", (24, 18, 500, 34));
        SendMessageW(
            heading,
            WM_SETFONT,
            WPARAM(self.heading_font.0 as usize),
            LPARAM(1),
        );
        self.label("Dites votre phrase. ChatGPT s'ouvre, vous parlez, puis 3 secondes de silence envoient le message.", (24, 58, 820, 40));
        self.label("Déclencheurs vocaux", (24, 105, 300, 24));
        self.control(
            w!("LISTBOX"),
            "",
            WS_BORDER
                | WS_VSCROLL
                | WS_TABSTOP
                | WINDOW_STYLE(LBS_NOTIFY as u32 | LBS_NOINTEGRALHEIGHT as u32),
            LIST,
            (24, 132, 804, 124),
        );
        self.label("Phrase à prononcer", (24, 270, 350, 22));
        self.edit("", PHRASE, (24, 296, 394, 28));
        self.label("Action", (438, 270, 390, 22));
        self.control(
            w!("COMBOBOX"),
            "",
            WS_TABSTOP | WS_VSCROLL | WINDOW_STYLE(CBS_DROPDOWNLIST as u32),
            ACTION,
            (438, 296, 390, 180),
        );
        for action in [
            "Dicter à ChatGPT",
            "Ouvrir ChatGPT",
            "Ouvrir un site",
            "Lancer une application",
        ] {
            SendMessageW(
                self.child(ACTION),
                CB_ADDSTRING,
                WPARAM(0),
                LPARAM(wide(action).as_ptr() as isize),
            );
        }
        self.label(
            "Cible (adresse HTTP(S) ou chemin .exe, selon l'action)",
            (24, 334, 804, 22),
        );
        self.edit("", TARGET, (24, 360, 804, 28));
        self.button("Appliquer à la sélection", APPLY, (24, 402, 212, 32));
        self.button("Ajouter", ADD, (250, 402, 120, 32));
        self.button("Supprimer", DELETE, (384, 402, 120, 32));
        self.button("Enregistrer", SAVE, (680, 402, 148, 32));
        self.label("Langue de reconnaissance", (24, 452, 380, 22));
        self.control(
            w!("COMBOBOX"),
            "",
            WS_TABSTOP | WS_VSCROLL | WINDOW_STYLE(CBS_DROPDOWNLIST as u32),
            LANGUAGE,
            (24, 478, 394, 140),
        );
        for language in ["Français (France)", "Anglais (États-Unis)"] {
            SendMessageW(
                self.child(LANGUAGE),
                CB_ADDSTRING,
                WPARAM(0),
                LPARAM(wide(language).as_ptr() as isize),
            );
        }
        SendMessageW(
            self.child(LANGUAGE),
            CB_SETCURSEL,
            WPARAM(if self.settings.language == 0x0409 {
                1
            } else {
                0
            }),
            LPARAM(0),
        );
        self.label("Seuil du silence (0.001 à 0.5)", (438, 452, 390, 22));
        self.edit(
            &self.settings.voice_threshold.to_string(),
            THRESHOLD,
            (438, 478, 390, 28),
        );
        self.label(
            "Chemin ChatGPT.exe (facultatif si ChatGPT est déjà lancé)",
            (24, 520, 804, 22),
        );
        self.edit(&self.settings.chatgpt_path, CHAT_PATH, (24, 546, 804, 28));
        self.button("Démarrer l'écoute", START, (24, 592, 190, 36));
        self.button("Arrêter", STOP, (228, 592, 118, 36));
        self.button("Diagnostic", DIAGNOSTIC, (360, 592, 140, 36));
        self.button("Arrière-plan", HIDE, (548, 592, 176, 36));
        self.button("Quitter", EXIT, (738, 592, 90, 36));
        self.control(
            w!("EDIT"),
            "Prêt. Enregistrez vos réglages puis démarrez l'écoute.",
            WS_BORDER
                | WS_VSCROLL
                | WINDOW_STYLE(ES_MULTILINE as u32 | ES_READONLY as u32 | ES_AUTOVSCROLL as u32),
            STATUS,
            (24, 648, 804, 78),
        );
        self.label("Fermer masque la fenêtre. Icône de notification : ouvrir, arrêter ou quitter. Déclencheurs locaux, sans clé API.", (24, 740, 804, 38));
        self.refresh_list(Some(0));
        self.update_controls(false);
    }

    unsafe fn selected(&self) -> Option<usize> {
        let index = SendMessageW(self.child(LIST), LB_GETCURSEL, WPARAM(0), LPARAM(0)).0;
        (index >= 0).then_some(index as usize)
    }

    unsafe fn refresh_list(&self, selected: Option<usize>) {
        SendMessageW(self.child(LIST), LB_RESETCONTENT, WPARAM(0), LPARAM(0));
        for trigger in &self.settings.triggers {
            let text = format!("{}     —     {}", trigger.phrase, trigger.action.label());
            SendMessageW(
                self.child(LIST),
                LB_ADDSTRING,
                WPARAM(0),
                LPARAM(wide(&text).as_ptr() as isize),
            );
        }
        if let Some(index) = selected.filter(|index| *index < self.settings.triggers.len()) {
            SendMessageW(self.child(LIST), LB_SETCURSEL, WPARAM(index), LPARAM(0));
        }
        self.load_selection();
    }

    unsafe fn load_selection(&self) {
        let Some(trigger) = self
            .selected()
            .and_then(|index| self.settings.triggers.get(index))
        else {
            return;
        };
        self.set_text(PHRASE, &trigger.phrase);
        self.set_text(TARGET, trigger.action.target());
        let index = match trigger.action {
            Action::ChatGptDictation => 0,
            Action::OpenChatGpt => 1,
            Action::OpenUrl(_) => 2,
            Action::Launch(_) => 3,
        };
        SendMessageW(self.child(ACTION), CB_SETCURSEL, WPARAM(index), LPARAM(0));
        self.target_enabled();
    }

    unsafe fn target_enabled(&self) {
        let index = SendMessageW(self.child(ACTION), CB_GETCURSEL, WPARAM(0), LPARAM(0)).0;
        EnableWindow(self.child(TARGET), index >= 2 && self.worker.is_none());
    }

    unsafe fn update_controls(&self, listening: bool) {
        for id in [
            PHRASE, ACTION, APPLY, ADD, DELETE, SAVE, START, LANGUAGE, THRESHOLD, CHAT_PATH,
        ] {
            EnableWindow(self.child(id), !listening);
        }
        EnableWindow(self.child(STOP), listening);
        self.target_enabled();
    }

    unsafe fn read_settings(&self) -> Result<Settings> {
        let mut settings = self.settings.clone();
        settings.language =
            if SendMessageW(self.child(LANGUAGE), CB_GETCURSEL, WPARAM(0), LPARAM(0)).0 == 1 {
                0x0409
            } else {
                0x040c
            };
        settings.voice_threshold = self
            .text(THRESHOLD)
            .trim()
            .replace(',', ".")
            .parse()
            .map_err(|_| "Seuil du silence invalide.")?;
        settings.chatgpt_path = self.text(CHAT_PATH).trim().trim_matches('"').into();
        settings.validate()?;
        Ok(settings)
    }

    unsafe fn apply_trigger(&mut self, add: bool) -> Result<()> {
        let target = self.text(TARGET).trim().trim_matches('"').to_string();
        let action = match SendMessageW(self.child(ACTION), CB_GETCURSEL, WPARAM(0), LPARAM(0)).0 {
            0 => Action::ChatGptDictation,
            1 => Action::OpenChatGpt,
            2 => Action::OpenUrl(target),
            3 => Action::Launch(target),
            _ => return Err("Choisissez une action.".into()),
        };
        let trigger = Trigger {
            phrase: self.text(PHRASE).trim().into(),
            action,
        };
        let mut settings = self.settings.clone();
        let index = if add {
            settings.triggers.push(trigger);
            settings.triggers.len() - 1
        } else {
            let index = self.selected().ok_or("Sélectionnez un déclencheur.")?;
            settings.triggers[index] = trigger;
            index
        };
        settings.validate()?;
        self.settings = settings;
        self.refresh_list(Some(index));
        self.set_text(
            STATUS,
            "Déclencheur modifié. Cliquez sur Enregistrer pour conserver vos changements.",
        );
        Ok(())
    }

    unsafe fn notify(&mut self, message: NOTIFY_ICON_MESSAGE, text: &str) -> bool {
        let mut data = NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: self.window,
            uID: 1,
            uFlags: NIF_ICON | NIF_MESSAGE | NIF_TIP,
            uCallbackMessage: TRAY_MESSAGE,
            hIcon: LoadIconW(None, IDI_APPLICATION).unwrap_or_default(),
            ..Default::default()
        };
        for (target, source) in data.szTip.iter_mut().take(127).zip(text.encode_utf16()) {
            *target = source;
        }
        Shell_NotifyIconW(message, &data).as_bool()
    }

    unsafe fn hide(&mut self) -> Result<()> {
        if !self.tray {
            self.tray = self.notify(NIM_ADD, "VoiceTrigger — ouvrir avec un double-clic");
        }
        if !self.tray {
            return Err("Icône de notification indisponible. La fenêtre reste ouverte.".into());
        }
        ShowWindow(self.window, SW_HIDE);
        Ok(())
    }

    unsafe fn show(&self) {
        ShowWindow(self.window, SW_RESTORE);
        SetForegroundWindow(self.window);
    }

    unsafe fn command(&mut self, id: u16) -> Result<()> {
        match id {
            APPLY => self.apply_trigger(false)?,
            ADD => self.apply_trigger(true)?,
            DELETE => {
                let index = self.selected().ok_or("Sélectionnez un déclencheur.")?;
                if self.settings.triggers.len() == 1 {
                    return Err("Conservez au moins un déclencheur.".into());
                }
                self.settings.triggers.remove(index);
                self.refresh_list(Some(index.min(self.settings.triggers.len() - 1)));
                self.set_text(STATUS, "Déclencheur supprimé. Cliquez sur Enregistrer.");
            }
            SAVE | START => {
                self.apply_trigger(false)?;
                self.settings = self.read_settings()?;
                self.settings.save(&self.path)?;
                self.set_text(STATUS, "Configuration enregistrée.");
                if id == START && self.worker.is_none() {
                    self.worker = Some(Worker::start(self.settings.clone(), self.events.clone()));
                    self.update_controls(true);
                    self.set_text(STATUS, "Initialisation du moteur vocal Windows…");
                    if !self.tray {
                        self.tray = self.notify(NIM_ADD, "VoiceTrigger — écoute en cours");
                    }
                }
            }
            STOP => {
                if let Some(worker) = &self.worker {
                    worker.stop();
                    self.set_text(STATUS, "Arrêt demandé. Annulation de la dictée en cours…");
                }
            }
            DIAGNOSTIC if !self.diagnostic_active => {
                let settings = self.read_settings()?;
                self.diagnostic_active = true;
                EnableWindow(self.child(DIAGNOSTIC), false);
                let events = self.events.clone();
                std::thread::spawn(move || {
                    let result = (|| {
                        let _com = super::Com::new()?;
                        Ok::<_, String>(format!(
                            "{}\r\n\r\n{}",
                            super::speech::diagnostic()?,
                            super::chatgpt::diagnostic(&settings)?
                        ))
                    })();
                    let _ = events.send(Event::Status(
                        result.unwrap_or_else(|error| format!("Diagnostic : {error}")),
                    ));
                    let _ = events.send(Event::DiagnosticDone);
                });
            }
            HIDE => self.hide()?,
            OPEN => self.show(),
            EXIT => {
                if let Some(worker) = &self.worker {
                    self.exit_when_stopped = true;
                    worker.stop();
                    self.set_text(STATUS, "Annulation en cours avant de quitter…");
                } else {
                    PostMessageW(self.window, QUIT_MESSAGE, WPARAM(0), LPARAM(0));
                }
            }
            _ => {}
        }
        Ok(())
    }

    unsafe fn drain_events(&mut self) {
        while let Ok(event) = self.receiver.try_recv() {
            match event {
                Event::Status(text) => {
                    self.set_text(STATUS, &text);
                    if text.starts_with("Erreur") {
                        self.show();
                    }
                }
                Event::Stopped => {
                    self.worker = None;
                    self.update_controls(false);
                    self.notify(NIM_MODIFY, "VoiceTrigger — écoute arrêtée");
                    if self.exit_when_stopped {
                        PostMessageW(self.window, QUIT_MESSAGE, WPARAM(0), LPARAM(0));
                    }
                }
                Event::DiagnosticDone => {
                    self.diagnostic_active = false;
                    EnableWindow(self.child(DIAGNOSTIC), true);
                }
            }
        }
    }
}

unsafe extern "system" fn procedure(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if message == WM_NCCREATE {
        let create = &*(lparam.0 as *const CREATESTRUCTW);
        SetWindowLongPtrW(window, GWLP_USERDATA, create.lpCreateParams as isize);
    }
    let pointer = GetWindowLongPtrW(window, GWLP_USERDATA) as *mut App;
    if pointer.is_null() {
        return DefWindowProcW(window, message, wparam, lparam);
    }
    match message {
        WM_COMMAND => {
            let id = (wparam.0 & 0xffff) as u16;
            let notification = (wparam.0 >> 16) as u32;
            // Text changes can synchronously re-enter this procedure. Ignore them
            // before borrowing App, and never hold App across a native modal loop.
            if notification != 0
                && !(id == LIST && notification == LBN_SELCHANGE
                    || id == ACTION && notification == CBN_SELCHANGE)
            {
                return LRESULT(0);
            }
            let app = &mut *pointer;
            if id == LIST && notification == LBN_SELCHANGE {
                app.load_selection();
            } else if id == ACTION && notification == CBN_SELCHANGE {
                app.target_enabled();
            } else if notification == 0 {
                if let Err(error) = app.command(id) {
                    app.set_text(STATUS, &error);
                }
            }
            LRESULT(0)
        }
        EVENT_MESSAGE => {
            (*pointer).drain_events();
            LRESULT(0)
        }
        SHOW_MESSAGE => {
            (*pointer).show();
            LRESULT(0)
        }
        QUIT_MESSAGE => {
            DestroyWindow(window);
            LRESULT(0)
        }
        WM_CLOSE => {
            let app = &mut *pointer;
            if let Err(error) = app.hide() {
                app.set_text(STATUS, &error);
            }
            LRESULT(0)
        }
        WM_SIZE if wparam.0 == SIZE_MINIMIZED as usize => {
            let _ = (*pointer).hide();
            LRESULT(0)
        }
        TRAY_MESSAGE => {
            if lparam.0 as u32 == WM_LBUTTONDBLCLK {
                (*pointer).show();
            }
            if lparam.0 as u32 == WM_RBUTTONUP {
                if let Ok(menu) = CreatePopupMenu() {
                    AppendMenuW(menu, MF_STRING, OPEN as usize, w!("Ouvrir VoiceTrigger"));
                    AppendMenuW(menu, MF_STRING, STOP as usize, w!("Arrêter l'écoute"));
                    AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
                    AppendMenuW(menu, MF_STRING, EXIT as usize, w!("Quitter"));
                    let mut point = POINT::default();
                    GetCursorPos(&mut point);
                    SetForegroundWindow(window);
                    let command = TrackPopupMenu(
                        menu,
                        TPM_RIGHTBUTTON | TPM_RETURNCMD,
                        point.x,
                        point.y,
                        0,
                        window,
                        None,
                    );
                    if command.0 != 0 {
                        PostMessageW(window, WM_COMMAND, WPARAM(command.0 as usize), LPARAM(0));
                    }
                    DestroyMenu(menu);
                }
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            let app = &mut *pointer;
            if app.tray {
                app.notify(NIM_DELETE, "");
            }
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(window, message, wparam, lparam),
    }
}

pub fn run(settings: Settings, path: PathBuf) -> Result<()> {
    struct Instance(HANDLE);
    impl Drop for Instance {
        fn drop(&mut self) {
            unsafe {
                let _ = CloseHandle(self.0);
            }
        }
    }
    unsafe {
        let handle = windows::Win32::System::Threading::CreateMutexW(
            None,
            false,
            w!("Local\\VoiceTrigger.Singleton"),
        )
        .map_err(|e| e.to_string())?;
        let already_running = GetLastError() == ERROR_ALREADY_EXISTS;
        let _instance = Instance(handle);
        let instance = GetModuleHandleW(None).map_err(|e| e.to_string())?;
        let class = w!("VoiceTriggerWindow");
        if already_running {
            let existing = FindWindowW(class, PCWSTR::null());
            if existing.0 != 0 {
                PostMessageW(existing, SHOW_MESSAGE, WPARAM(0), LPARAM(0));
            }
            return Ok(());
        }
        let window_class = WNDCLASSW {
            lpfnWndProc: Some(procedure),
            hInstance: instance,
            lpszClassName: class,
            hCursor: LoadCursorW(None, IDC_ARROW).map_err(|e| e.to_string())?,
            hbrBackground: HBRUSH((COLOR_BTNFACE.0 + 1) as isize),
            ..Default::default()
        };
        if RegisterClassW(&window_class) == 0 {
            return Err(windows::core::Error::from_win32().to_string());
        }
        let font = CreateFontW(
            -16,
            0,
            0,
            0,
            400,
            0,
            0,
            0,
            DEFAULT_CHARSET.0 as u32,
            OUT_DEFAULT_PRECIS.0 as u32,
            CLIP_DEFAULT_PRECIS.0 as u32,
            CLEARTYPE_QUALITY.0 as u32,
            0,
            w!("Segoe UI"),
        );
        let heading_font = CreateFontW(
            -27,
            0,
            0,
            0,
            600,
            0,
            0,
            0,
            DEFAULT_CHARSET.0 as u32,
            OUT_DEFAULT_PRECIS.0 as u32,
            CLIP_DEFAULT_PRECIS.0 as u32,
            CLEARTYPE_QUALITY.0 as u32,
            0,
            w!("Segoe UI"),
        );
        let (sender, receiver) = mpsc::channel();
        let mut app = Box::new(App {
            window: HWND(0),
            settings,
            path,
            font,
            heading_font,
            worker: None,
            events: Events { sender, window: 0 },
            receiver,
            diagnostic_active: false,
            tray: false,
            exit_when_stopped: false,
        });
        let window = CreateWindowExW(
            WS_EX_CONTROLPARENT,
            class,
            w!("VoiceTrigger — déclencheurs vocaux"),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            870,
            830,
            None,
            None,
            instance,
            Some(app.as_mut() as *mut App as *const _),
        );
        if window.0 == 0 {
            return Err(windows::core::Error::from_win32().to_string());
        }
        app.window = window;
        app.events.window = window.0;
        app.initialize();
        ShowWindow(window, SW_SHOW);
        let mut message = MSG::default();
        loop {
            let result = GetMessageW(&mut message, None, 0, 0).0;
            if result == -1 {
                return Err(windows::core::Error::from_win32().to_string());
            }
            if result == 0 {
                break;
            }
            if !IsDialogMessageW(window, &message).as_bool() {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
        if let Some(worker) = &app.worker {
            worker.stop();
        }
        DeleteObject(font);
        DeleteObject(heading_font);
    }
    Ok(())
}
