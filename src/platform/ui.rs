use super::{
    engine::{Event, Events, Worker},
    wide, Result,
};
use crate::config::{Action, Settings, Trigger};
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
    InitCommonControlsEx, BST_CHECKED, DRAWITEMSTRUCT, EM_SCROLLCARET, EM_SETSEL,
    ICC_PROGRESS_CLASS, INITCOMMONCONTROLSEX, MEASUREITEMSTRUCT, PBM_SETPOS, PBM_SETRANGE32,
};
use windows::Win32::UI::HiDpi::*;
use windows::Win32::UI::Input::KeyboardAndMouse::{EnableWindow, IsWindowEnabled, SetFocus};
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
const BRAND: u16 = 300;
const BRAND_HINT: u16 = 301;
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

#[derive(Clone, Copy, PartialEq, Eq)]
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

#[derive(Clone, Copy, PartialEq, Eq)]
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
struct ViewState {
    page: Cell<Page>,
    diagnostic_tab: Cell<DiagnosticTab>,
    listening: Cell<bool>,
}
#[derive(Clone, Copy)]
enum Scope {
    Global,
    Section(Page),
    Diagnostic(DiagnosticTab),
}
struct Control {
    id: u16,
    handle: HWND,
    scope: Scope,
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
    menu: HMENU,
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
        self.view.page.set(page);
        let (title, hint) = match page {
            Page::Triggers => ("Triggers", "Choose a phrase and the action it runs."),
            Page::Settings => (
                "Settings",
                "Configure local recognition and ChatGPT dictation.",
            ),
            Page::Diagnostics => (
                "Diagnostics",
                "Monitor recognition and check the ChatGPT connection.",
            ),
            Page::Credits => ("Credits", "Project information and open-source licenses."),
        };
        self.set_text(PAGE_TITLE, title);
        self.set_text(PAGE_HINT, hint);
        CheckMenuRadioItem(
            GetSubMenu(self.menu, 1),
            NAV_TRIGGERS as u32,
            NAV_CREDITS as u32,
            page.id() as u32,
            MF_BYCOMMAND.0,
        );
        self.layout();
        self.update_debug();
        SetFocus(self.child(page.id()));
    }
    unsafe fn append_log(&mut self, text: &str) {
        self.journal.append(self.started.elapsed(), text);
        self.set_text(LOG, &self.journal.render());
        let end = GetWindowTextLengthW(self.child(LOG)) as usize;
        SendMessageW(
            self.child(LOG),
            EM_SETSEL,
            WPARAM(end),
            LPARAM(end as isize),
        );
        SendMessageW(self.child(LOG), EM_SCROLLCARET, WPARAM(0), LPARAM(0));
    }

    unsafe fn debug_enabled(&self) -> bool {
        IsWindowVisible(self.window).as_bool()
            && self.view.page.get() == Page::Diagnostics
            && self.view.diagnostic_tab.get() == DiagnosticTab::Activity
            && SendMessageW(self.child(DEBUG), BM_GETCHECK, WPARAM(0), LPARAM(0)).0
                == BST_CHECKED.0 as isize
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
            SendMessageW(self.child(METER), PBM_SETPOS, WPARAM(0), LPARAM(0));
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
    }

    unsafe fn update_controls(&self, listening: bool) {
        for id in [
            LIST, PHRASE, ACTION, APPLY, ADD, DELETE, SAVE, START, LANGUAGE, THRESHOLD, CHAT_PATH,
        ] {
            EnableWindow(self.child(id), !listening);
        }
        EnableWindow(self.child(STOP), listening);
        self.target_enabled();
        self.view.listening.set(listening);
        self.set_text(
            MODE,
            if listening {
                "Listening"
            } else {
                "Not listening"
            },
        );
        EnableMenuItem(
            GetSubMenu(self.menu, 0),
            SAVE as u32,
            MF_BYCOMMAND | if listening { MF_GRAYED } else { MF_ENABLED },
        );
        EnableMenuItem(
            GetSubMenu(self.menu, 0),
            START as u32,
            MF_BYCOMMAND | if listening { MF_GRAYED } else { MF_ENABLED },
        );
        EnableMenuItem(
            GetSubMenu(self.menu, 0),
            STOP as u32,
            MF_BYCOMMAND | if listening { MF_ENABLED } else { MF_GRAYED },
        );
        InvalidateRect(self.window, None, false);
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
        ShowWindow(self.window, SW_RESTORE);
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
                self.update_debug();
            }
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
                    SendMessageW(self.child(METER), PBM_SETPOS, WPARAM(0), LPARAM(0));
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
                        SendMessageW(
                            self.child(METER),
                            PBM_SETPOS,
                            WPARAM(level as usize),
                            LPARAM(0),
                        );
                    }
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
            item.itemHeight = state.theme.px(68) as u32;
            LRESULT(1)
        }
        WM_GETMINMAXINFO => {
            let limits = &mut *(lparam.0 as *mut MINMAXINFO);
            let mut rect = RECT {
                right: state.theme.px(980),
                bottom: state.theme.px(640),
                ..Default::default()
            };
            AdjustWindowRectExForDpi(
                &mut rect,
                WS_OVERLAPPEDWINDOW,
                true,
                WS_EX_CONTROLPARENT,
                state.theme.dpi.get(),
            );
            limits.ptMinTrackSize = POINT {
                x: rect.right - rect.left,
                y: rect.bottom - rect.top,
            };
            LRESULT(0)
        }
        WM_DPICHANGED => {
            if let Ok(app) = state.app.try_borrow_mut() {
                let old = state.theme.change_dpi((wparam.0 & 0xffff) as u32);
                for control in &app.controls {
                    SendMessageW(
                        control.handle,
                        WM_SETFONT,
                        WPARAM(state.theme.font(control.font).0 as usize),
                        LPARAM(1),
                    );
                    if control.id == LIST {
                        SendMessageW(
                            control.handle,
                            LB_SETITEMHEIGHT,
                            WPARAM(0),
                            LPARAM(state.theme.px(68) as isize),
                        );
                    }
                }
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
                for font in old {
                    DeleteObject(font);
                }
            }
            LRESULT(0)
        }
        WM_COMMAND => {
            let id = (wparam.0 & 0xffff) as u16;
            let notification = (wparam.0 >> 16) as u32;
            let from_control = lparam.0 != 0;
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

unsafe fn menus() -> Result<HMENU> {
    let bar = CreateMenu().map_err(|e| e.to_string())?;
    let file = CreatePopupMenu().map_err(|e| e.to_string())?;
    AppendMenuW(
        file,
        MF_STRING,
        SAVE as usize,
        w!("&Save configuration\tCtrl+S"),
    );
    AppendMenuW(file, MF_SEPARATOR, 0, PCWSTR::null());
    AppendMenuW(file, MF_STRING, START as usize, w!("Start listening"));
    AppendMenuW(file, MF_STRING, STOP as usize, w!("Stop listening"));
    AppendMenuW(file, MF_STRING, HIDE as usize, w!("Hide to tray"));
    AppendMenuW(file, MF_SEPARATOR, 0, PCWSTR::null());
    AppendMenuW(file, MF_STRING, EXIT as usize, w!("&Quit"));
    AppendMenuW(bar, MF_POPUP, file.0 as usize, w!("&File"));
    let view = CreatePopupMenu().map_err(|e| e.to_string())?;
    for (id, label) in [
        (NAV_TRIGGERS, "&Triggers\tCtrl+1"),
        (NAV_SETTINGS, "&Settings\tCtrl+2"),
        (NAV_DIAGNOSTICS, "&Diagnostics\tCtrl+3"),
        (NAV_CREDITS, "&Credits\tCtrl+4"),
    ] {
        AppendMenuW(view, MF_STRING, id as usize, PCWSTR(wide(label).as_ptr()));
    }
    AppendMenuW(bar, MF_POPUP, view.0 as usize, w!("&View"));
    let help = CreatePopupMenu().map_err(|e| e.to_string())?;
    AppendMenuW(
        help,
        MF_STRING,
        DOCUMENTATION as usize,
        w!("&Documentation"),
    );
    AppendMenuW(help, MF_STRING, REPOSITORY as usize, w!("Source code"));
    AppendMenuW(help, MF_STRING, NAV_CREDITS as usize, w!("&Credits\tF1"));
    AppendMenuW(bar, MF_POPUP, help.0 as usize, w!("&Help"));
    CheckMenuRadioItem(
        view,
        NAV_TRIGGERS as u32,
        NAV_CREDITS as u32,
        NAV_TRIGGERS as u32,
        MF_BYCOMMAND.0,
    );
    Ok(bar)
}

struct Desktop {
    state: Box<WindowState>,
    class: PCWSTR,
    instance: HMODULE,
}
impl Desktop {
    unsafe fn new(settings: Settings, path: PathBuf, class: PCWSTR, title: &str) -> Result<Self> {
        let theme = Rc::new(Theme::new(GetDpiForSystem()));
        let view = Rc::new(ViewState {
            page: Cell::new(Page::Triggers),
            diagnostic_tab: Cell::new(DiagnosticTab::Activity),
            listening: Cell::new(false),
        });
        let (sender, receiver) = mpsc::channel();
        let menu = menus()?;
        let state = Box::new(WindowState {
            events_pending: Cell::new(false),
            theme: theme.clone(),
            view: view.clone(),
            app: RefCell::new(App {
                window: HWND(0),
                menu,
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
            dwICC: ICC_PROGRESS_CLASS,
        });
        let window_class = WNDCLASSW {
            lpfnWndProc: Some(procedure),
            hInstance: instance,
            lpszClassName: class,
            hCursor: LoadCursorW(None, IDC_ARROW).map_err(|e| e.to_string())?,
            hIcon: LoadIconW(None, IDI_APPLICATION).unwrap_or_default(),
            ..Default::default()
        };
        if RegisterClassW(&window_class) == 0 {
            DestroyMenu(menu);
            return Err(windows::core::Error::from_win32().to_string());
        }
        let mut outer = RECT {
            right: theme.px(1100),
            bottom: theme.px(740),
            ..Default::default()
        };
        AdjustWindowRectExForDpi(
            &mut outer,
            WS_OVERLAPPEDWINDOW,
            true,
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
            menu,
            instance,
            Some(state.as_ref() as *const _ as *const _),
        );
        if window.0 == 0 {
            DestroyMenu(menu);
            UnregisterClassW(class, instance);
            return Err(windows::core::Error::from_win32().to_string());
        }
        {
            let mut app = state.app.borrow_mut();
            app.window = window;
            app.events.window = window.0;
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
    let handle = CreateAcceleratorTableW(&[
        ACCEL {
            fVirt: FVIRTKEY | FCONTROL,
            key: b'S' as u16,
            cmd: SAVE,
        },
        ACCEL {
            fVirt: FVIRTKEY | FCONTROL,
            key: b'1' as u16,
            cmd: NAV_TRIGGERS,
        },
        ACCEL {
            fVirt: FVIRTKEY | FCONTROL,
            key: b'2' as u16,
            cmd: NAV_SETTINGS,
        },
        ACCEL {
            fVirt: FVIRTKEY | FCONTROL,
            key: b'3' as u16,
            cmd: NAV_DIAGNOSTICS,
        },
        ACCEL {
            fVirt: FVIRTKEY | FCONTROL,
            key: b'4' as u16,
            cmd: NAV_CREDITS,
        },
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
