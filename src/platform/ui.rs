use super::{
    engine::{Event, Events, Worker},
    wide, Result,
};
use crate::config::{Action, Appearance, Settings, Trigger};
use crate::journal::Journal;
use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::mpsc::{self, Receiver};
use std::time::Instant;
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Controls::{
    InitCommonControlsEx, SetWindowTheme, DRAWITEMSTRUCT, ICC_STANDARD_CLASSES,
    INITCOMMONCONTROLSEX, MEASUREITEMSTRUCT, ODT_COMBOBOX, WM_MOUSELEAVE,
};
use windows::Win32::UI::HiDpi::*;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    EnableWindow, GetFocus, IsWindowEnabled, SetFocus, TrackMouseEvent, TME_LEAVE, TRACKMOUSEEVENT,
};
use windows::Win32::UI::Shell::*;
use windows::Win32::UI::WindowsAndMessaging::*;
mod layout;
mod style;
#[cfg(test)]
mod tests;
use style::{Font, Theme};

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
const DEBUG: u16 = 119;
const LEVEL: u16 = 120;
const LOG: u16 = 121;
const CLEAR_LOG: u16 = 122;
const METER: u16 = 123;
const REPORT: u16 = 124;
const NAV_TRIGGERS: u16 = 200;
const NAV_SETTINGS: u16 = 201;
const NAV_DIAGNOSTICS: u16 = 202;
const NAV_CREDITS: u16 = 203;
const REPOSITORY: u16 = 210;
const DOCUMENTATION: u16 = 211;
const ACTIVITY_TAB: u16 = 220;
const CHECKS_TAB: u16 = 221;
const THEME_LIGHT: u16 = 230;
const THEME_DARK: u16 = 231;
const BRAND: u16 = 300;
const VERSION: u16 = 302;
const MODE: u16 = 303;
const PAGE_TITLE: u16 = 304;
const PAGE_HINT: u16 = 305;
const LIST_TITLE: u16 = 310;
const EDITOR_TITLE: u16 = 311;
const PHRASE_LABEL: u16 = 312;
const ACTION_LABEL: u16 = 313;
const TARGET_LABEL: u16 = 314;
const TARGET_HINT: u16 = 315;
const RECOGNITION_TITLE: u16 = 320;
const LANGUAGE_LABEL: u16 = 321;
const THRESHOLD_LABEL: u16 = 322;
const LANGUAGE_HINT: u16 = 323;
const THRESHOLD_HINT: u16 = 324;
const CHATGPT_TITLE: u16 = 325;
const PATH_LABEL: u16 = 326;
const PATH_HINT: u16 = 327;
const INPUT_TITLE: u16 = 330;
const LOG_TITLE: u16 = 331;
const CHECKS_TITLE: u16 = 332;
const CHECKS_HINT: u16 = 333;
const CREDIT_NAME: u16 = 340;
const CREDIT_VERSION: u16 = 341;
const CREDIT_OWNER: u16 = 342;
const CREDIT_BUILT: u16 = 343;
const CREDIT_LIBRARIES: u16 = 344;
const LICENSE_TITLE: u16 = 345;
const LICENSE_TEXT: u16 = 346;
const APPEARANCE_TITLE: u16 = 350;
const THEME_LABEL: u16 = 351;
const THEME_HINT: u16 = 352;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Page {
    Triggers,
    Settings,
    Diagnostics,
    Credits,
}
impl Page {
    fn id(self) -> u16 {
        NAV_TRIGGERS + self as u16
    }
    fn from_id(id: u16) -> Option<Self> {
        match id {
            NAV_TRIGGERS => Some(Self::Triggers),
            NAV_SETTINGS => Some(Self::Settings),
            NAV_DIAGNOSTICS => Some(Self::Diagnostics),
            NAV_CREDITS => Some(Self::Credits),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum DiagnosticTab {
    Activity,
    Checks,
}
impl DiagnosticTab {
    fn id(self) -> u16 {
        if self == Self::Activity {
            ACTIVITY_TAB
        } else {
            CHECKS_TAB
        }
    }
}

/// Paint-only view state. It never borrows the application, so native
/// painting and subclass callbacks can read it during reentrant messages.
struct ViewState {
    page: Cell<Page>,
    diagnostic_tab: Cell<DiagnosticTab>,
    listening: Cell<bool>,
    live_debug: Cell<bool>,
    level: Cell<u8>,
    hover: Cell<isize>,
}
struct Control {
    id: u16,
    handle: HWND,
    font: Font,
}

// Paint callbacks use independent resources. RefCell rejects accidental nested
// application mutation during synchronous native notifications or modal loops.
struct WindowState {
    events_pending: Cell<bool>,
    app: RefCell<App>,
    theme: Rc<Theme>,
    view: Rc<ViewState>,
}
struct App {
    window: HWND,
    settings: Settings,
    path: PathBuf,
    theme: Rc<Theme>,
    view: Rc<ViewState>,
    controls: Vec<Control>,
    initialized: bool,
    worker: Option<Worker>,
    events: Events,
    receiver: Receiver<Event>,
    diagnostic_active: bool,
    tray: bool,
    exit_when_stopped: bool,
    journal: Journal,
    started: Instant,
}

impl App {
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
    unsafe fn select_page(&self, page: Page) {
        let previous = self.view.page.replace(page);
        let (title, hint) = match page {
            Page::Triggers => ("Triggers", "Choose a phrase and the action it runs."),
            Page::Settings => (
                "Settings",
                "Appearance, local recognition and ChatGPT dictation.",
            ),
            Page::Diagnostics => (
                "Diagnostics",
                "Monitor recognition and check the ChatGPT connection.",
            ),
            Page::Credits => ("Credits", "Project information and open-source licenses."),
        };
        self.set_text(PAGE_TITLE, title);
        self.set_text(PAGE_HINT, hint);
        self.layout();
        for id in [previous.id(), page.id()] {
            InvalidateRect(self.child(id), None, false);
        }
        self.update_debug();
        SetFocus(self.child(page.id()));
    }
    unsafe fn append_log(&mut self, text: &str) {
        self.journal.append(self.started.elapsed(), text);
        self.set_text(LOG, &self.journal.render());
        let end = GetWindowTextLengthW(self.child(LOG)) as usize;
        SendMessageW(
            self.child(LOG),
            windows::Win32::UI::Controls::EM_SETSEL,
            WPARAM(end),
            LPARAM(end as isize),
        );
        SendMessageW(
            self.child(LOG),
            windows::Win32::UI::Controls::EM_SCROLLCARET,
            WPARAM(0),
            LPARAM(0),
        );
    }

    unsafe fn debug_enabled(&self) -> bool {
        IsWindowVisible(self.window).as_bool()
            && self.view.page.get() == Page::Diagnostics
            && self.view.diagnostic_tab.get() == DiagnosticTab::Activity
            && self.view.live_debug.get()
    }

    unsafe fn set_level(&self, level: u8) {
        if self.view.level.replace(level) != level {
            InvalidateRect(self.child(METER), None, false);
        }
    }

    unsafe fn update_debug(&self) {
        let enabled = self.debug_enabled();
        if let Some(worker) = &self.worker {
            worker.set_debug(enabled);
        } else {
            self.set_text(LEVEL, "Microphone: stopped");
        }
        if !enabled && self.worker.is_some() {
            self.set_text(LEVEL, "Live debug paused; trigger listening is unchanged.");
            self.set_level(0);
        }
    }

    unsafe fn selected(&self) -> Option<usize> {
        let index = SendMessageW(self.child(LIST), LB_GETCURSEL, WPARAM(0), LPARAM(0)).0;
        (index >= 0).then_some(index as usize)
    }

    unsafe fn refresh_list(&self, selected: Option<usize>) {
        SendMessageW(self.child(LIST), LB_RESETCONTENT, WPARAM(0), LPARAM(0));
        for trigger in &self.settings.triggers {
            let text = format!("{}\t{}", trigger.phrase, trigger.action.label());
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
        self.set_text(
            TARGET_LABEL,
            match index {
                2 => "Website address",
                3 => "Executable path",
                _ => "Target (not required)",
            },
        );
        self.set_text(
            TARGET_HINT,
            match index {
                2 => "Enter a complete HTTP or HTTPS address.",
                3 => "Choose an existing .exe file without arguments.",
                _ => "This action does not require a target.",
            },
        );
        InvalidateRect(self.window, None, false);
    }

    unsafe fn update_controls(&self, listening: bool) {
        let focus = GetFocus();
        let had_focus = focus == self.child(START) || focus == self.child(STOP);
        for id in [
            LIST, PHRASE, ACTION, APPLY, ADD, DELETE, SAVE, START, LANGUAGE, THRESHOLD, CHAT_PATH,
        ] {
            EnableWindow(self.child(id), !listening);
        }
        EnableWindow(self.child(STOP), listening);
        self.view.listening.set(listening);
        self.target_enabled();
        self.set_text(
            MODE,
            if listening {
                "Listening"
            } else {
                "Not listening"
            },
        );
        self.layout();
        if had_focus {
            SetFocus(self.child(if listening { STOP } else { START }));
        }
        RedrawWindow(self.window, None, None, RDW_INVALIDATE | RDW_ALLCHILDREN);
    }

    /// Applies the current palette to the title bar, scroll bars and controls.
    unsafe fn apply_theme(&self) {
        self.theme.apply_frame(self.window);
        let name = if self.theme.palette().dark {
            w!("DarkMode_Explorer")
        } else {
            w!("Explorer")
        };
        for id in [LIST, LOG, REPORT, LICENSE_TEXT] {
            let _ = SetWindowTheme(self.child(id), name, PCWSTR::null());
        }
        RedrawWindow(
            self.window,
            None,
            None,
            RDW_INVALIDATE | RDW_ERASE | RDW_ALLCHILDREN | RDW_FRAME,
        );
    }

    /// Applies a theme immediately and stores only that preference, leaving
    /// unsaved trigger and setting edits out of the configuration file.
    unsafe fn set_appearance(&mut self, appearance: Appearance) -> Result<()> {
        if self.settings.appearance == appearance {
            return Ok(());
        }
        self.settings.appearance = appearance;
        self.theme.set_appearance(appearance);
        self.apply_theme();
        let mut stored = Settings::load(&self.path)?;
        stored.appearance = appearance;
        stored.save(&self.path)?;
        self.set_text(
            STATUS,
            match appearance {
                Appearance::Light => "Light theme applied and saved.",
                Appearance::Dark => "Dark theme applied and saved.",
            },
        );
        Ok(())
    }

    unsafe fn change_dpi(&self, dpi: u32) {
        let (fonts, icons) = self.theme.change_dpi(dpi);
        for control in &self.controls {
            SendMessageW(
                control.handle,
                WM_SETFONT,
                WPARAM(self.theme.font(control.font).0 as usize),
                LPARAM(1),
            );
        }
        self.size_lists();
        self.set_icons();
        for font in fonts {
            DeleteObject(font);
        }
        for icon in icons {
            DestroyIcon(icon);
        }
        if self.tray {
            self.notify(NIM_MODIFY, "VoiceTrigger");
        }
    }

    unsafe fn set_icons(&self) {
        for (kind, class, large) in [
            (ICON_SMALL, GCLP_HICONSM, false),
            (ICON_BIG, GCLP_HICON, true),
        ] {
            let icon = self.theme.icon(large).0;
            // The class keeps no reference to icons retired after a DPI change.
            SetClassLongPtrW(self.window, class, icon);
            SendMessageW(self.window, WM_SETICON, WPARAM(kind as usize), LPARAM(icon));
        }
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
            .map_err(|_| "Invalid silence threshold.")?;
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
            _ => return Err("Choose an action.".into()),
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
            let index = self.selected().ok_or("Select a trigger.")?;
            settings.triggers[index] = trigger;
            index
        };
        settings.validate()?;
        self.settings = settings;
        self.refresh_list(Some(index));
        self.set_text(
            STATUS,
            "Trigger updated. Save configuration to keep your changes.",
        );
        Ok(())
    }

    unsafe fn notify(&self, message: NOTIFY_ICON_MESSAGE, text: &str) -> bool {
        let mut data = NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: self.window,
            uID: 1,
            uFlags: NIF_ICON | NIF_MESSAGE | NIF_TIP,
            uCallbackMessage: TRAY_MESSAGE,
            hIcon: self.theme.icon(false),
            ..Default::default()
        };
        for (target, source) in data.szTip.iter_mut().take(127).zip(text.encode_utf16()) {
            *target = source;
        }
        Shell_NotifyIconW(message, &data).as_bool()
    }

    unsafe fn hide(&mut self) -> Result<()> {
        if !self.tray {
            self.tray = self.notify(NIM_ADD, "VoiceTrigger - double-click to open");
        }
        if !self.tray {
            return Err(
                "The notification icon is unavailable. The window will remain visible.".into(),
            );
        }
        ShowWindow(self.window, SW_HIDE);
        self.update_debug();
        Ok(())
    }

    unsafe fn show(&self) {
        // Restore minimized windows, but keep a maximized window maximized.
        ShowWindow(
            self.window,
            if IsIconic(self.window).as_bool() {
                SW_RESTORE
            } else {
                SW_SHOW
            },
        );
        SetForegroundWindow(self.window);
        self.layout();
        self.update_debug();
    }

    unsafe fn command(&mut self, id: u16) -> Result<()> {
        if self.worker.is_some() && matches!(id, APPLY | ADD | DELETE | SAVE | START) {
            return Err("Stop listening before editing the configuration.".into());
        }
        if let Some(page) = Page::from_id(id) {
            self.select_page(page);
            return Ok(());
        }
        match id {
            ACTIVITY_TAB | CHECKS_TAB => {
                self.view.diagnostic_tab.set(if id == ACTIVITY_TAB {
                    DiagnosticTab::Activity
                } else {
                    DiagnosticTab::Checks
                });
                self.layout();
                for tab in [ACTIVITY_TAB, CHECKS_TAB] {
                    InvalidateRect(self.child(tab), None, false);
                }
                self.update_debug();
            }
            THEME_LIGHT => self.set_appearance(Appearance::Light)?,
            THEME_DARK => self.set_appearance(Appearance::Dark)?,
            REPOSITORY => super::chatgpt::open_url("https://github.com/TheGeogeo/VoiceTrigger")?,
            DOCUMENTATION => super::chatgpt::open_url(
                "https://github.com/TheGeogeo/VoiceTrigger/blob/main/README.md",
            )?,
            APPLY => self.apply_trigger(false)?,
            ADD => self.apply_trigger(true)?,
            DELETE => {
                let index = self.selected().ok_or("Select a trigger.")?;
                if self.settings.triggers.len() == 1 {
                    return Err("Keep at least one trigger.".into());
                }
                self.settings.triggers.remove(index);
                self.refresh_list(Some(index.min(self.settings.triggers.len() - 1)));
                self.set_text(
                    STATUS,
                    "Trigger removed. Save configuration to keep the change.",
                );
            }
            SAVE | START => {
                self.apply_trigger(false)?;
                self.settings = self.read_settings()?;
                self.settings.save(&self.path)?;
                self.set_text(STATUS, "Settings saved.");
                if id == START && self.worker.is_none() {
                    self.worker = Some(Worker::start(
                        self.settings.clone(),
                        self.events.clone(),
                        self.debug_enabled(),
                    ));
                    self.update_controls(true);
                    self.set_text(STATUS, "Starting the Windows speech recognizer...");
                    self.set_text(LEVEL, "Waiting for live audio events…");
                    if !self.tray {
                        self.tray = self.notify(NIM_ADD, "VoiceTrigger - listening");
                    }
                }
            }
            STOP => {
                if let Some(worker) = &self.worker {
                    worker.stop();
                    self.set_text(STATUS, "Stop requested. Cancelling any active dictation...");
                }
            }
            DIAGNOSTIC if !self.diagnostic_active => {
                let settings = self.read_settings()?;
                self.diagnostic_active = true;
                self.set_text(
                    REPORT,
                    "Checking recognition engines and ChatGPT controls...",
                );
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
                    let _ = events.send(Event::DiagnosticReport(
                        result.unwrap_or_else(|error| format!("Diagnostic: {error}")),
                    ));
                    let _ = events.send(Event::DiagnosticDone);
                });
            }
            HIDE => self.hide()?,
            DEBUG => {
                self.view.live_debug.set(!self.view.live_debug.get());
                InvalidateRect(self.child(DEBUG), None, false);
                self.update_debug();
                self.append_log(if self.debug_enabled() {
                    "Live debug enabled."
                } else {
                    "Live debug disabled. Trigger listening remains active."
                });
            }
            CLEAR_LOG => {
                self.journal.clear();
                self.set_text(LOG, "");
            }
            OPEN => self.show(),
            EXIT => {
                if let Some(worker) = &self.worker {
                    self.exit_when_stopped = true;
                    worker.stop();
                    self.set_text(STATUS, "Cancelling before quitting...");
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
                    self.append_log(&text);
                    if text.starts_with("Error") {
                        self.show();
                    }
                }
                Event::Stopped => {
                    self.worker = None;
                    self.update_controls(false);
                    self.set_text(LEVEL, "Microphone: stopped");
                    self.set_level(0);
                    self.notify(NIM_MODIFY, "VoiceTrigger - listening stopped");
                    if self.exit_when_stopped {
                        PostMessageW(self.window, QUIT_MESSAGE, WPARAM(0), LPARAM(0));
                    }
                }
                Event::DiagnosticReport(report) => {
                    self.set_text(REPORT, &report);
                    self.set_text(
                        STATUS,
                        "System checks completed. See Diagnostics > System checks.",
                    );
                    self.append_log("System checks completed.");
                }
                Event::DiagnosticDone => {
                    self.diagnostic_active = false;
                    EnableWindow(self.child(DIAGNOSTIC), true);
                }
                Event::Trace(text) => self.append_log(&text),
                Event::AudioLevel(level) => {
                    if self.debug_enabled() {
                        self.set_text(
                            LEVEL,
                            &format!(
                                "Input level: {level}% — receiving audio ({:.1}s)",
                                self.started.elapsed().as_secs_f32()
                            ),
                        );
                        self.set_level(level);
                    }
                }
            }
        }
    }
}

/// Adds hover feedback to owner-drawn buttons and paints closed selects so
/// they match text fields. It reads only paint state, never the application.
unsafe extern "system" fn subclass(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _id: usize,
    select: usize,
) -> LRESULT {
    let pointer = GetWindowLongPtrW(GetParent(window), GWLP_USERDATA) as *const WindowState;
    if !pointer.is_null() {
        let state = &*pointer;
        let hover = state.view.hover.get() == window.0;
        match message {
            WM_MOUSEMOVE if !hover => {
                let previous = state.view.hover.replace(window.0);
                if previous != 0 {
                    InvalidateRect(HWND(previous), None, false);
                }
                let mut track = TRACKMOUSEEVENT {
                    cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
                    dwFlags: TME_LEAVE,
                    hwndTrack: window,
                    dwHoverTime: 0,
                };
                TrackMouseEvent(&mut track);
                InvalidateRect(window, None, false);
            }
            WM_MOUSELEAVE if hover => {
                state.view.hover.set(0);
                InvalidateRect(window, None, false);
            }
            WM_ERASEBKGND if select != 0 => return LRESULT(1),
            WM_PAINT if select != 0 => {
                if wparam.0 != 0 {
                    state
                        .theme
                        .paint_select(window, HDC(wparam.0 as isize), hover);
                } else {
                    let mut paint = PAINTSTRUCT::default();
                    let dc = BeginPaint(window, &mut paint);
                    state.theme.paint_select(window, dc, hover);
                    EndPaint(window, &paint);
                }
                return LRESULT(0);
            }
            WM_PRINTCLIENT if select != 0 => {
                state
                    .theme
                    .paint_select(window, HDC(wparam.0 as isize), hover);
                return LRESULT(0);
            }
            WM_NCDESTROY => {
                if hover {
                    state.view.hover.set(0);
                }
                RemoveWindowSubclass(window, Some(subclass), 1);
            }
            _ => {}
        }
    }
    DefSubclassProc(window, message, wparam, lparam)
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
    let pointer = GetWindowLongPtrW(window, GWLP_USERDATA) as *const WindowState;
    if pointer.is_null() {
        return DefWindowProcW(window, message, wparam, lparam);
    }
    let state = &*pointer;
    let result = match message {
        WM_ERASEBKGND => LRESULT(1),
        WM_PAINT => {
            state.theme.paint(window, &state.view);
            LRESULT(0)
        }
        WM_CTLCOLORSTATIC | WM_CTLCOLOREDIT | WM_CTLCOLORLISTBOX | WM_CTLCOLORBTN => {
            let child = HWND(lparam.0);
            let brush = state.theme.control_color(
                HDC(wparam.0 as isize),
                GetDlgCtrlID(child) as u16,
                IsWindowEnabled(child).as_bool(),
                message == WM_CTLCOLOREDIT,
            );
            LRESULT(brush.0)
        }
        WM_DRAWITEM => {
            state
                .theme
                .draw_item(&*(lparam.0 as *const DRAWITEMSTRUCT), &state.view);
            LRESULT(1)
        }
        WM_MEASUREITEM => {
            let item = &mut *(lparam.0 as *mut MEASUREITEMSTRUCT);
            item.itemHeight = state.theme.px(if item.CtlType == ODT_COMBOBOX {
                if item.itemID == u32::MAX {
                    layout::FIELD - 6
                } else {
                    layout::OPTION_HEIGHT
                }
            } else {
                layout::ROW_HEIGHT
            }) as u32;
            LRESULT(1)
        }
        WM_GETMINMAXINFO => {
            let limits = &mut *(lparam.0 as *mut MINMAXINFO);
            let mut rect = RECT {
                right: state.theme.px(layout::MIN_WIDTH),
                bottom: state.theme.px(layout::MIN_HEIGHT),
                ..Default::default()
            };
            AdjustWindowRectExForDpi(
                &mut rect,
                WS_OVERLAPPEDWINDOW,
                false,
                WS_EX_CONTROLPARENT,
                state.theme.dpi.get(),
            );
            // Never require more than the monitor's work area, so the whole
            // window stays reachable on small or highly scaled displays.
            let mut monitor = MONITORINFO {
                cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                ..Default::default()
            };
            let (mut width, mut height) = (rect.right - rect.left, rect.bottom - rect.top);
            if GetMonitorInfoW(
                MonitorFromWindow(window, MONITOR_DEFAULTTONEAREST),
                &mut monitor,
            )
            .as_bool()
            {
                width = width.min(monitor.rcWork.right - monitor.rcWork.left);
                height = height.min(monitor.rcWork.bottom - monitor.rcWork.top);
            }
            limits.ptMinTrackSize = POINT {
                x: width,
                y: height,
            };
            LRESULT(0)
        }
        WM_DPICHANGED => {
            if let Ok(app) = state.app.try_borrow_mut() {
                app.change_dpi((wparam.0 & 0xffff) as u32);
                let rect = &*(lparam.0 as *const RECT);
                SetWindowPos(
                    window,
                    None,
                    rect.left,
                    rect.top,
                    rect.right - rect.left,
                    rect.bottom - rect.top,
                    SWP_NOZORDER | SWP_NOACTIVATE,
                );
                app.layout();
            }
            LRESULT(0)
        }
        WM_COMMAND => {
            let id = (wparam.0 & 0xffff) as u16;
            let notification = (wparam.0 >> 16) as u32;
            let from_control = lparam.0 != 0;
            if from_control && notification != 0 {
                // Field focus changes only repaint frames; they never borrow App.
                match id {
                    PHRASE | TARGET | THRESHOLD | CHAT_PATH
                        if matches!(notification, EN_SETFOCUS | EN_KILLFOCUS) =>
                    {
                        InvalidateRect(window, None, false);
                    }
                    ACTION | LANGUAGE => {
                        InvalidateRect(HWND(lparam.0), None, false);
                    }
                    _ => {}
                }
            }
            // Edit controls synchronously notify their parent. Do not borrow the
            // application again while a command is already updating those controls.
            if from_control
                && notification != 0
                && !(id == LIST && notification == LBN_SELCHANGE
                    || id == ACTION && notification == CBN_SELCHANGE)
            {
                return LRESULT(0);
            }
            if let Ok(mut app) = state.app.try_borrow_mut() {
                if id == LIST && notification == LBN_SELCHANGE {
                    app.load_selection();
                } else if id == ACTION && notification == CBN_SELCHANGE {
                    app.target_enabled();
                } else if !from_control || notification == 0 {
                    if let Err(error) = app.command(id) {
                        app.set_text(STATUS, &error);
                        app.append_log(&error);
                    }
                }
            }
            LRESULT(0)
        }
        EVENT_MESSAGE => {
            if let Ok(mut app) = state.app.try_borrow_mut() {
                app.drain_events();
            } else {
                state.events_pending.set(true);
            }
            LRESULT(0)
        }
        SHOW_MESSAGE => {
            if let Ok(app) = state.app.try_borrow() {
                app.show();
            }
            LRESULT(0)
        }
        QUIT_MESSAGE => {
            DestroyWindow(window);
            LRESULT(0)
        }
        WM_CLOSE => {
            if let Ok(mut app) = state.app.try_borrow_mut() {
                if let Err(error) = app.hide() {
                    app.set_text(STATUS, &error);
                }
            }
            LRESULT(0)
        }
        WM_SIZE => {
            if let Ok(mut app) = state.app.try_borrow_mut() {
                if wparam.0 == SIZE_MINIMIZED as usize {
                    let _ = app.hide();
                } else {
                    app.layout();
                }
            }
            LRESULT(0)
        }
        TRAY_MESSAGE => {
            if lparam.0 as u32 == WM_LBUTTONDBLCLK {
                if let Ok(app) = state.app.try_borrow() {
                    app.show();
                }
            }
            if lparam.0 as u32 == WM_RBUTTONUP {
                if let Ok(menu) = CreatePopupMenu() {
                    AppendMenuW(menu, MF_STRING, OPEN as usize, w!("Open VoiceTrigger"));
                    AppendMenuW(menu, MF_STRING, STOP as usize, w!("Stop listening"));
                    AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
                    AppendMenuW(menu, MF_STRING, EXIT as usize, w!("Quit"));
                    let mut point = POINT::default();
                    GetCursorPos(&mut point);
                    SetForegroundWindow(window);
                    // Do not hold an application borrow across the native menu loop.
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
            if let Ok(mut app) = state.app.try_borrow_mut() {
                if app.tray {
                    app.notify(NIM_DELETE, "");
                    app.tray = false;
                }
                if let Some(worker) = &app.worker {
                    worker.stop();
                }
            }
            PostQuitMessage(0);
            LRESULT(0)
        }
        WM_NCDESTROY => {
            SetWindowLongPtrW(window, GWLP_USERDATA, 0);
            DefWindowProcW(window, message, wparam, lparam)
        }
        _ => DefWindowProcW(window, message, wparam, lparam),
    };
    // A native modal operation can consume the final worker notification while
    // an outer callback owns App. Repost once that callback releases its borrow.
    if state.events_pending.get() && state.app.try_borrow_mut().is_ok() {
        state.events_pending.set(false);
        PostMessageW(window, EVENT_MESSAGE, WPARAM(0), LPARAM(0));
    }
    result
}

struct Desktop {
    state: Box<WindowState>,
    class: PCWSTR,
    instance: HMODULE,
}
impl Desktop {
    unsafe fn new(settings: Settings, path: PathBuf, class: PCWSTR, title: &str) -> Result<Self> {
        let theme = Rc::new(Theme::new(GetDpiForSystem(), settings.appearance));
        let view = Rc::new(ViewState {
            page: Cell::new(Page::Triggers),
            diagnostic_tab: Cell::new(DiagnosticTab::Activity),
            listening: Cell::new(false),
            live_debug: Cell::new(true),
            level: Cell::new(0),
            hover: Cell::new(0),
        });
        let (sender, receiver) = mpsc::channel();
        let state = Box::new(WindowState {
            events_pending: Cell::new(false),
            theme: theme.clone(),
            view: view.clone(),
            app: RefCell::new(App {
                window: HWND(0),
                settings,
                path,
                theme: theme.clone(),
                view,
                controls: Vec::new(),
                initialized: false,
                worker: None,
                events: Events { sender, window: 0 },
                receiver,
                diagnostic_active: false,
                tray: false,
                exit_when_stopped: false,
                journal: Journal::default(),
                started: Instant::now(),
            }),
        });
        let instance = GetModuleHandleW(None).map_err(|e| e.to_string())?;
        InitCommonControlsEx(&INITCOMMONCONTROLSEX {
            dwSize: std::mem::size_of::<INITCOMMONCONTROLSEX>() as u32,
            dwICC: ICC_STANDARD_CLASSES,
        });
        let window_class = WNDCLASSW {
            lpfnWndProc: Some(procedure),
            hInstance: instance,
            lpszClassName: class,
            hCursor: LoadCursorW(None, IDC_ARROW).map_err(|e| e.to_string())?,
            hIcon: theme.icon(true),
            ..Default::default()
        };
        if RegisterClassW(&window_class) == 0 {
            return Err(windows::core::Error::from_win32().to_string());
        }
        let mut outer = RECT {
            right: theme.px(layout::DEFAULT_WIDTH),
            bottom: theme.px(layout::DEFAULT_HEIGHT),
            ..Default::default()
        };
        AdjustWindowRectExForDpi(
            &mut outer,
            WS_OVERLAPPEDWINDOW,
            false,
            WS_EX_CONTROLPARENT,
            theme.dpi.get(),
        );
        let mut work = RECT::default();
        let mut width = outer.right - outer.left;
        let mut height = outer.bottom - outer.top;
        let (mut x, mut y) = (CW_USEDEFAULT, CW_USEDEFAULT);
        if SystemParametersInfoW(
            SPI_GETWORKAREA,
            0,
            Some((&mut work as *mut RECT).cast()),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )
        .as_bool()
        {
            width = width.min(work.right - work.left - 24);
            height = height.min(work.bottom - work.top - 24);
            x = work.left + (work.right - work.left - width) / 2;
            y = work.top + (work.bottom - work.top - height) / 2;
        }
        let window = CreateWindowExW(
            WS_EX_CONTROLPARENT,
            class,
            PCWSTR(wide(title).as_ptr()),
            WS_OVERLAPPEDWINDOW | WS_CLIPCHILDREN,
            x,
            y,
            width,
            height,
            None,
            None,
            instance,
            Some(state.as_ref() as *const _ as *const _),
        );
        if window.0 == 0 {
            UnregisterClassW(class, instance);
            return Err(windows::core::Error::from_win32().to_string());
        }
        {
            let mut app = state.app.borrow_mut();
            app.window = window;
            app.events.window = window.0;
            let dpi = GetDpiForWindow(window);
            if dpi != 0 && dpi != theme.dpi.get() {
                // Created on a monitor whose scale differs from the primary one.
                let (fonts, icons) = theme.change_dpi(dpi);
                for font in fonts {
                    DeleteObject(font);
                }
                for icon in icons {
                    DestroyIcon(icon);
                }
            }
            app.set_icons();
            app.initialize();
        }
        Ok(Self {
            state,
            class,
            instance,
        })
    }
    fn window(&self) -> HWND {
        self.state.app.borrow().window
    }
}
impl Drop for Desktop {
    fn drop(&mut self) {
        unsafe {
            let window = self.window();
            if IsWindow(window).as_bool() {
                DestroyWindow(window);
            }
            UnregisterClassW(self.class, self.instance);
        }
    }
}

struct AcceleratorTable(HACCEL);
impl Drop for AcceleratorTable {
    fn drop(&mut self) {
        unsafe {
            DestroyAcceleratorTable(self.0);
        }
    }
}
unsafe fn accelerators() -> Result<AcceleratorTable> {
    let control = |key: u8, cmd: u16| ACCEL {
        fVirt: FVIRTKEY | FCONTROL,
        key: key as u16,
        cmd,
    };
    let handle = CreateAcceleratorTableW(&[
        control(b'S', SAVE),
        control(b'Q', EXIT),
        control(b'1', NAV_TRIGGERS),
        control(b'2', NAV_SETTINGS),
        control(b'3', NAV_DIAGNOSTICS),
        control(b'4', NAV_CREDITS),
        ACCEL {
            fVirt: FVIRTKEY,
            key: 0x70,
            cmd: NAV_CREDITS,
        },
    ])
    .map_err(|e| e.to_string())?;
    Ok(AcceleratorTable(handle))
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
        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        let handle = windows::Win32::System::Threading::CreateMutexW(
            None,
            false,
            w!("Local\\VoiceTrigger.Singleton"),
        )
        .map_err(|e| e.to_string())?;
        let already_running = GetLastError() == ERROR_ALREADY_EXISTS;
        let _instance = Instance(handle);
        let class = w!("VoiceTriggerWindow");
        if already_running {
            let existing = FindWindowW(class, PCWSTR::null());
            if existing.0 != 0 {
                PostMessageW(existing, SHOW_MESSAGE, WPARAM(0), LPARAM(0));
            }
            return Ok(());
        }
        let desktop = Desktop::new(settings, path, class, "VoiceTrigger")?;
        let window = desktop.window();
        let accelerators = accelerators()?;
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
            if TranslateAcceleratorW(window, accelerators.0, &message) == 0
                && !IsDialogMessageW(window, &message).as_bool()
            {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
    }
    Ok(())
}
