use super::*;

/// A rectangle in logical (96 DPI) pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    pub const fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self { x, y, w, h }
    }
    pub fn right(self) -> i32 {
        self.x + self.w
    }
    pub fn bottom(self) -> i32 {
        self.y + self.h
    }
    #[cfg(test)]
    pub fn contains(self, other: Rect) -> bool {
        other.x >= self.x
            && other.y >= self.y
            && other.right() <= self.right()
            && other.bottom() <= self.bottom()
    }
    #[cfg(test)]
    pub fn overlaps(self, other: Rect) -> bool {
        self.x < other.right()
            && other.x < self.right()
            && self.y < other.bottom()
            && other.y < self.bottom()
    }
}

pub(super) const MIN_WIDTH: i32 = 960;
pub(super) const MIN_HEIGHT: i32 = 600;
pub(super) const DEFAULT_WIDTH: i32 = 1100;
pub(super) const DEFAULT_HEIGHT: i32 = 740;
pub(super) const SIDEBAR: i32 = 232;
pub(super) const ROW_HEIGHT: i32 = 56;
pub(super) const OPTION_HEIGHT: i32 = 32;
pub(super) const FIELD: i32 = 36;
pub(super) const BRAND_ICON: Rect = Rect::new(20, 22, 32, 32);
const LEFT: i32 = 256;
const MARGIN: i32 = 32;
const TOP: i32 = 100;
const FOOTER: i32 = 72;
const GAP: i32 = 16;
const PAD: i32 = 20;
const SETTING_ROW: i32 = 68;

/// One placement plan per page, shared by control positioning and painting so
/// that cards, frames and controls cannot drift apart.
#[derive(Default)]
pub(super) struct Plan {
    pub cards: Vec<Rect>,
    pub tracks: Vec<Rect>,
    pub dividers: Vec<Rect>,
    pub items: Vec<(u16, Rect)>,
}

impl Plan {
    fn add(&mut self, id: u16, rect: Rect) {
        self.items.push((id, rect));
    }
    pub fn get(&self, id: u16) -> Option<Rect> {
        self.items
            .iter()
            .find(|(item, _)| *item == id)
            .map(|(_, rect)| *rect)
    }
    fn header(&mut self, title: u16, card: Rect, reserved: i32) {
        self.add(
            title,
            Rect::new(
                card.x + PAD,
                card.y + 16,
                card.w - 2 * PAD - reserved,
                FIELD,
            ),
        );
    }
    fn segments(&mut self, track: Rect, ids: &[(u16, i32)]) {
        self.tracks.push(track);
        let mut x = track.x + 3;
        for (id, width) in ids {
            self.add(*id, Rect::new(x, track.y + 3, *width, track.h - 6));
            x += width;
        }
    }
}

/// Status dot beside the listening label in the sidebar.
pub(super) fn status_dot(height: i32) -> Rect {
    Rect::new(24, height - 82, 8, 8)
}

pub(super) fn plan(
    page: Page,
    tab: DiagnosticTab,
    listening: bool,
    width: i32,
    height: i32,
) -> Plan {
    let mut plan = Plan::default();
    let inner = SIDEBAR - 24;
    plan.add(BRAND, Rect::new(62, 18, SIDEBAR - 74, 22));
    plan.add(VERSION, Rect::new(62, 40, SIDEBAR - 74, 18));
    for (index, id) in [NAV_TRIGGERS, NAV_SETTINGS, NAV_DIAGNOSTICS, NAV_CREDITS]
        .into_iter()
        .enumerate()
    {
        plan.add(id, Rect::new(12, 84 + index as i32 * 44, inner, 40));
    }
    let footer = height - FOOTER;
    let row = footer + 18;
    plan.add(MODE, Rect::new(40, row - 34, SIDEBAR - 52, 20));
    plan.add(
        if listening { STOP } else { START },
        Rect::new(12, row, inner, FIELD),
    );

    let width = width - LEFT - MARGIN;
    plan.add(PAGE_TITLE, Rect::new(LEFT, 20, width, 36));
    plan.add(PAGE_HINT, Rect::new(LEFT, 58, width, 22));
    plan.dividers.push(Rect::new(LEFT, footer, width, 1));
    let hide = Rect::new(LEFT + width - 128, row, 128, FIELD);
    let save = Rect::new(hide.x - 8 - 164, row, 164, FIELD);
    plan.add(SAVE, save);
    plan.add(HIDE, hide);
    plan.add(STATUS, Rect::new(LEFT, row, save.x - LEFT - 24, FIELD));

    let content = Rect::new(LEFT, TOP, width, footer - 20 - TOP);
    match page {
        Page::Triggers => triggers(&mut plan, content),
        Page::Settings => settings(&mut plan, content),
        Page::Diagnostics => diagnostics(&mut plan, content, tab),
        Page::Credits => credits(&mut plan, content),
    }
    plan
}

fn triggers(plan: &mut Plan, content: Rect) {
    let list_width = (content.w * 34 / 100).clamp(232, 340);
    let list = Rect::new(content.x, content.y, list_width, content.h);
    let editor = Rect::new(
        list.right() + GAP,
        content.y,
        content.w - list_width - GAP,
        content.h,
    );
    plan.cards.extend([list, editor]);
    plan.header(LIST_TITLE, list, 0);
    plan.add(
        LIST,
        Rect::new(list.x + 8, list.y + 60, list.w - 16, list.h - 68),
    );
    plan.header(EDITOR_TITLE, editor, 0);
    let (x, w, y) = (editor.x + PAD, editor.w - 2 * PAD, editor.y);
    for (index, (label, field)) in [
        (PHRASE_LABEL, PHRASE),
        (ACTION_LABEL, ACTION),
        (TARGET_LABEL, TARGET),
    ]
    .into_iter()
    .enumerate()
    {
        let top = y + 64 + index as i32 * 76;
        plan.add(label, Rect::new(x, top, w, 20));
        plan.add(field, Rect::new(x, top + 24, w, FIELD));
    }
    plan.add(TARGET_HINT, Rect::new(x, y + 284, w, 32));
    // Actions follow the fields they apply to rather than the card bottom.
    let buttons = y + 336;
    plan.add(APPLY, Rect::new(x, buttons, 136, FIELD));
    plan.add(ADD, Rect::new(x + 144, buttons, 120, FIELD));
    plan.add(
        DELETE,
        Rect::new(editor.right() - PAD - 104, buttons, 104, FIELD),
    );
}

/// Label, description, optional control (None: theme switch) and control width.
type SettingRow = (u16, u16, Option<u16>, i32);

fn settings(plan: &mut Plan, content: Rect) {
    let path_width = (content.w * 45 / 100).clamp(280, 520);
    let sections: [(u16, &[SettingRow]); 3] = [
        (APPEARANCE_TITLE, &[(THEME_LABEL, THEME_HINT, None, 200)]),
        (
            RECOGNITION_TITLE,
            &[
                (LANGUAGE_LABEL, LANGUAGE_HINT, Some(LANGUAGE), 260),
                (THRESHOLD_LABEL, THRESHOLD_HINT, Some(THRESHOLD), 140),
            ],
        ),
        (
            CHATGPT_TITLE,
            &[(PATH_LABEL, PATH_HINT, Some(CHAT_PATH), path_width)],
        ),
    ];
    let mut y = content.y;
    for (title, rows) in sections {
        plan.add(title, Rect::new(content.x, y, content.w, 20));
        let card = Rect::new(
            content.x,
            y + 28,
            content.w,
            rows.len() as i32 * SETTING_ROW,
        );
        plan.cards.push(card);
        for (index, (label, hint, control, control_width)) in rows.iter().enumerate() {
            let top = card.y + index as i32 * SETTING_ROW;
            if index > 0 {
                plan.dividers
                    .push(Rect::new(card.x + PAD, top, card.w - 2 * PAD, 1));
            }
            let text = card.w - 2 * PAD - control_width - 24;
            plan.add(*label, Rect::new(card.x + PAD, top + 12, text, 20));
            plan.add(*hint, Rect::new(card.x + PAD, top + 34, text, 32));
            let field = Rect::new(
                card.right() - PAD - control_width,
                top + 16,
                *control_width,
                FIELD,
            );
            match control {
                Some(id) => plan.add(*id, field),
                None => plan.segments(field, &[(THEME_LIGHT, 97), (THEME_DARK, 97)]),
            }
        }
        y = card.bottom() + GAP;
    }
}

fn diagnostics(plan: &mut Plan, content: Rect, tab: DiagnosticTab) {
    plan.segments(
        Rect::new(content.x, content.y, 260, FIELD),
        &[(ACTIVITY_TAB, 120), (CHECKS_TAB, 134)],
    );
    let body = Rect::new(content.x, content.y + 52, content.w, content.h - 52);
    let (x, w) = (body.x + PAD, body.w - 2 * PAD);
    if tab == DiagnosticTab::Activity {
        let microphone = Rect::new(body.x, body.y, body.w, 112);
        let log = Rect::new(
            body.x,
            microphone.bottom() + GAP,
            body.w,
            body.bottom() - microphone.bottom() - GAP,
        );
        plan.cards.extend([microphone, log]);
        plan.header(INPUT_TITLE, microphone, 168);
        plan.add(
            DEBUG,
            Rect::new(
                microphone.right() - PAD - 148,
                microphone.y + 16,
                148,
                FIELD,
            ),
        );
        plan.add(LEVEL, Rect::new(x, microphone.y + 58, w, 20));
        plan.add(METER, Rect::new(x, microphone.y + 86, w, 8));
        plan.header(LOG_TITLE, log, 120);
        plan.add(
            CLEAR_LOG,
            Rect::new(log.right() - PAD - 104, log.y + 16, 104, FIELD),
        );
        plan.add(LOG, Rect::new(x, log.y + 64, w, log.h - 64 - PAD));
    } else {
        plan.cards.push(body);
        plan.header(CHECKS_TITLE, body, 140);
        plan.add(
            DIAGNOSTIC,
            Rect::new(body.right() - PAD - 124, body.y + 16, 124, FIELD),
        );
        plan.add(CHECKS_HINT, Rect::new(x, body.y + 56, w, 20));
        plan.add(REPORT, Rect::new(x, body.y + 88, w, body.h - 88 - PAD));
    }
}

fn credits(plan: &mut Plan, content: Rect) {
    let about_width = (content.w * 36 / 100).clamp(256, 340);
    let about = Rect::new(content.x, content.y, about_width, content.h);
    let license = Rect::new(
        about.right() + GAP,
        content.y,
        content.w - about_width - GAP,
        content.h,
    );
    plan.cards.extend([about, license]);
    let (x, w, y) = (about.x + PAD, about.w - 2 * PAD, about.y);
    plan.add(CREDIT_NAME, Rect::new(x, y + 16, w, FIELD));
    plan.add(CREDIT_VERSION, Rect::new(x, y + 54, w, 20));
    plan.add(CREDIT_OWNER, Rect::new(x, y + 78, w, 40));
    plan.add(REPOSITORY, Rect::new(x, y + 130, w, FIELD));
    plan.add(DOCUMENTATION, Rect::new(x, y + 174, w, FIELD));
    plan.dividers.push(Rect::new(x, y + 228, w, 1));
    plan.add(CREDIT_BUILT, Rect::new(x, y + 244, w, 20));
    plan.add(CREDIT_LIBRARIES, Rect::new(x, y + 270, w, 84));
    plan.header(LICENSE_TITLE, license, 0);
    plan.add(
        LICENSE_TEXT,
        Rect::new(
            license.x + PAD,
            license.y + 64,
            license.w - 2 * PAD,
            license.h - 64 - PAD,
        ),
    );
}

/// Joins hard-wrapped license lines so the text flows with the panel width.
pub(super) fn reflow(text: &str) -> String {
    text.replace("\r\n", "\n")
        .split("\n\n")
        .map(|paragraph| paragraph.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|paragraph| !paragraph.is_empty())
        .collect::<Vec<_>>()
        .join("\r\n\r\n")
}

const SINGLE_LINE: WINDOW_STYLE = WINDOW_STYLE(0x200 | 0x80 | 0x4000); // SS_CENTERIMAGE | SS_NOPREFIX | SS_ENDELLIPSIS
const WRAPPED: WINDOW_STYLE = WINDOW_STYLE(0x80); // SS_LEFT | SS_NOPREFIX
const OWNER_DRAWN_STATIC: WINDOW_STYLE = WINDOW_STYLE(0x0d); // SS_OWNERDRAW

impl App {
    unsafe fn control(
        &mut self,
        id: u16,
        class: PCWSTR,
        text: &str,
        style: WINDOW_STYLE,
        font: Font,
    ) -> HWND {
        let handle = CreateWindowExW(
            WINDOW_EX_STYLE(0),
            class,
            PCWSTR(wide(text).as_ptr()),
            WS_CHILD | style,
            0,
            0,
            0,
            0,
            self.window,
            HMENU(id as isize),
            None,
            None,
        );
        SendMessageW(
            handle,
            WM_SETFONT,
            WPARAM(self.theme.font(font).0 as usize),
            LPARAM(0),
        );
        self.controls.push(Control { id, handle, font });
        handle
    }

    unsafe fn label(&mut self, id: u16, text: &str, font: Font) {
        self.control(id, w!("STATIC"), text, SINGLE_LINE, font);
    }

    unsafe fn paragraph(&mut self, id: u16, text: &str, font: Font) {
        self.control(id, w!("STATIC"), text, WRAPPED, font);
    }

    unsafe fn input(&mut self, id: u16, text: &str) {
        self.control(
            id,
            w!("EDIT"),
            text,
            WS_TABSTOP | WINDOW_STYLE(ES_AUTOHSCROLL as u32),
            Font::Body,
        );
    }

    unsafe fn well(&mut self, id: u16, text: &str) {
        self.control(
            id,
            w!("EDIT"),
            text,
            WS_TABSTOP
                | WS_VSCROLL
                | WINDOW_STYLE(ES_MULTILINE as u32 | ES_READONLY as u32 | ES_AUTOVSCROLL as u32),
            Font::Mono,
        );
    }

    unsafe fn button(&mut self, id: u16, text: &str) {
        let handle = self.control(
            id,
            w!("BUTTON"),
            text,
            WS_TABSTOP | WINDOW_STYLE(BS_OWNERDRAW as u32),
            Font::Body,
        );
        SetWindowSubclass(handle, Some(super::subclass), 1, 0);
    }

    unsafe fn select(&mut self, id: u16, options: &[&str], selected: usize) {
        let handle = self.control(
            id,
            w!("COMBOBOX"),
            "",
            WS_TABSTOP
                | WS_VSCROLL
                | WINDOW_STYLE(
                    CBS_DROPDOWNLIST as u32 | CBS_OWNERDRAWFIXED as u32 | CBS_HASSTRINGS as u32,
                ),
            Font::Body,
        );
        for option in options {
            SendMessageW(
                handle,
                CB_ADDSTRING,
                WPARAM(0),
                LPARAM(wide(option).as_ptr() as isize),
            );
        }
        SendMessageW(handle, CB_SETCURSEL, WPARAM(selected), LPARAM(0));
        SetWindowSubclass(handle, Some(super::subclass), 1, 1);
    }

    pub(super) unsafe fn initialize(&mut self) {
        self.label(BRAND, "VoiceTrigger", Font::Brand);
        self.label(
            VERSION,
            concat!("Version ", env!("CARGO_PKG_VERSION")),
            Font::Small,
        );
        self.label(PAGE_TITLE, "Triggers", Font::Heading);
        self.label(
            PAGE_HINT,
            "Choose a phrase and the action it runs.",
            Font::Body,
        );
        for (id, text) in [
            (NAV_TRIGGERS, "Triggers"),
            (NAV_SETTINGS, "Settings"),
            (NAV_DIAGNOSTICS, "Diagnostics"),
            (NAV_CREDITS, "Credits"),
        ] {
            self.button(id, text);
        }

        self.label(LIST_TITLE, "Configured triggers", Font::Label);
        self.control(
            LIST,
            w!("LISTBOX"),
            "",
            WS_TABSTOP
                | WS_VSCROLL
                | WINDOW_STYLE(
                    LBS_NOTIFY as u32
                        | LBS_NOINTEGRALHEIGHT as u32
                        | LBS_OWNERDRAWFIXED as u32
                        | LBS_HASSTRINGS as u32,
                ),
            Font::Body,
        );
        self.label(EDITOR_TITLE, "Trigger details", Font::Label);
        self.label(PHRASE_LABEL, "Spoken phrase", Font::Body);
        self.input(PHRASE, "");
        self.label(ACTION_LABEL, "Action", Font::Body);
        self.select(
            ACTION,
            &[
                "Dictate to ChatGPT",
                "Open ChatGPT",
                "Open a website",
                "Launch an application",
            ],
            0,
        );
        self.label(TARGET_LABEL, "Target", Font::Body);
        self.input(TARGET, "");
        self.paragraph(
            TARGET_HINT,
            "Used for a website address or an executable path.",
            Font::Small,
        );
        for (id, text) in [
            (APPLY, "Apply changes"),
            (ADD, "Add as new"),
            (DELETE, "Remove"),
        ] {
            self.button(id, text);
        }

        self.label(APPEARANCE_TITLE, "Appearance", Font::Label);
        self.label(THEME_LABEL, "Theme", Font::Body);
        self.paragraph(
            THEME_HINT,
            "Applies to the whole window and is saved immediately.",
            Font::Small,
        );
        self.button(THEME_LIGHT, "Light");
        self.button(THEME_DARK, "Dark");
        self.label(RECOGNITION_TITLE, "Recognition", Font::Label);
        self.label(LANGUAGE_LABEL, "Recognition language", Font::Body);
        self.paragraph(
            LANGUAGE_HINT,
            "Uses an installed Windows SAPI recognizer.",
            Font::Small,
        );
        self.select(
            LANGUAGE,
            &["French (France)", "English (United States)"],
            usize::from(self.settings.language == 0x0409),
        );
        self.label(THRESHOLD_LABEL, "Silence threshold", Font::Body);
        self.paragraph(
            THRESHOLD_HINT,
            "Speech level for the 3-second silence that submits dictation. 0.001 to 0.5; default 0.015.",
            Font::Small,
        );
        self.input(THRESHOLD, &self.settings.voice_threshold.to_string());
        self.label(CHATGPT_TITLE, "ChatGPT", Font::Label);
        self.label(PATH_LABEL, "Executable path", Font::Body);
        self.paragraph(
            PATH_HINT,
            "Optional. Leave blank to use the installed app.",
            Font::Small,
        );
        self.input(CHAT_PATH, &self.settings.chatgpt_path.clone());

        self.button(ACTIVITY_TAB, "Activity");
        self.button(CHECKS_TAB, "System checks");
        self.label(INPUT_TITLE, "Microphone", Font::Label);
        self.button(DEBUG, "Live debug");
        self.label(LEVEL, "Microphone: stopped", Font::Body);
        self.control(
            METER,
            w!("STATIC"),
            "Microphone level",
            OWNER_DRAWN_STATIC,
            Font::Small,
        );
        self.label(LOG_TITLE, "Event log", Font::Label);
        self.button(CLEAR_LOG, "Clear log");
        self.well(LOG, "");
        self.label(CHECKS_TITLE, "System checks", Font::Label);
        self.label(
            CHECKS_HINT,
            "Check installed recognition engines and accessible ChatGPT controls.",
            Font::Small,
        );
        self.button(DIAGNOSTIC, "Run checks");
        self.well(
            REPORT,
            "Run checks to inspect the recognition engine and ChatGPT connection.",
        );

        self.label(CREDIT_NAME, "VoiceTrigger", Font::Brand);
        self.label(
            CREDIT_VERSION,
            concat!("Version ", env!("CARGO_PKG_VERSION")),
            Font::Body,
        );
        self.paragraph(
            CREDIT_OWNER,
            "Created by TheGeogeo\r\nCopyright 2026 TheGeogeo",
            Font::Body,
        );
        self.button(REPOSITORY, "Source code");
        self.button(DOCUMENTATION, "Documentation");
        self.label(CREDIT_BUILT, "Built with", Font::Label);
        self.paragraph(
            CREDIT_LIBRARIES,
            "Rust\r\nwindows-rs 0.48.0\r\nSerde 1.0.188\r\nserde_json 1.0.105\r\nLibraries: MIT or Apache 2.0",
            Font::Small,
        );
        self.label(LICENSE_TITLE, "MIT license", Font::Label);
        self.well(LICENSE_TEXT, &reflow(include_str!("../../../LICENSE")));

        self.control(
            STATUS,
            w!("STATIC"),
            "Ready. Start listening to enable your voice triggers.",
            OWNER_DRAWN_STATIC,
            Font::Small,
        );
        self.button(SAVE, "Save configuration");
        self.button(HIDE, "Hide to tray");
        self.label(MODE, "Not listening", Font::Small);
        self.button(START, "Start listening");
        self.button(STOP, "Stop listening");

        self.append_log("Ready. Trigger and action events are kept in memory only.");
        self.size_lists();
        self.refresh_list(Some(0));
        self.initialized = true;
        self.apply_theme();
        self.update_controls(false);
    }

    /// Sets owner-drawn row heights and makes closed selects match text fields.
    pub(super) unsafe fn size_lists(&self) {
        SendMessageW(
            self.child(LIST),
            LB_SETITEMHEIGHT,
            WPARAM(0),
            LPARAM(self.theme.px(ROW_HEIGHT) as isize),
        );
        let target = self.theme.px(FIELD);
        for id in [ACTION, LANGUAGE] {
            let select = self.child(id);
            SendMessageW(
                select,
                CB_SETITEMHEIGHT,
                WPARAM(0),
                LPARAM(self.theme.px(OPTION_HEIGHT) as isize),
            );
            let mut field = target - 6;
            for _ in 0..2 {
                SendMessageW(
                    select,
                    CB_SETITEMHEIGHT,
                    WPARAM(usize::MAX),
                    LPARAM(field as isize),
                );
                let mut rect = RECT::default();
                GetWindowRect(select, &mut rect);
                let actual = rect.bottom - rect.top;
                if actual == target || actual <= 0 {
                    break;
                }
                field += target - actual;
            }
        }
    }

    pub(super) unsafe fn logical_size(&self) -> (i32, i32) {
        let mut rect = RECT::default();
        GetClientRect(self.window, &mut rect);
        let dpi = self.theme.dpi.get() as i32;
        (rect.right * 96 / dpi, rect.bottom * 96 / dpi)
    }

    pub(super) unsafe fn layout(&self) {
        if !self.initialized {
            return;
        }
        let (width, height) = self.logical_size();
        let plan = plan(
            self.view.page.get(),
            self.view.diagnostic_tab.get(),
            self.view.listening.get(),
            width,
            height,
        );
        let line = self.theme.line_height(Font::Body);
        for control in &self.controls {
            let Some(frame) = plan.get(control.id) else {
                // A hidden control receives no mouse-leave; clear its hover state.
                if self.view.hover.get() == control.handle.0 {
                    self.view.hover.set(0);
                }
                ShowWindow(control.handle, SW_HIDE);
                continue;
            };
            let mut rect = self.theme.rect(frame);
            match control.id {
                PHRASE | TARGET | THRESHOLD | CHAT_PATH => {
                    rect.left += self.theme.px(12);
                    rect.right -= self.theme.px(12);
                    rect.top += (rect.bottom - rect.top - line) / 2;
                    rect.bottom = rect.top + line;
                }
                LOG | REPORT | LICENSE_TEXT => {
                    rect.left += self.theme.px(12);
                    rect.right -= self.theme.px(4);
                    rect.top += self.theme.px(8);
                    rect.bottom -= self.theme.px(8);
                }
                ACTION | LANGUAGE => {
                    rect.bottom += self.theme.px(4 * OPTION_HEIGHT + 8);
                }
                _ => {}
            }
            SetWindowPos(
                control.handle,
                None,
                rect.left,
                rect.top,
                (rect.right - rect.left).max(1),
                (rect.bottom - rect.top).max(1),
                SWP_NOZORDER | SWP_NOACTIVATE,
            );
            ShowWindow(control.handle, SW_SHOWNA);
        }
        // Keep scrollable text aligned to complete font lines at every size.
        for id in [LOG, REPORT, LICENSE_TEXT] {
            let child = self.child(id);
            let line_height = self.theme.line_height(Font::Mono).max(1);
            let mut text_rect = RECT::default();
            GetClientRect(child, &mut text_rect);
            text_rect.right -= self.theme.px(8);
            text_rect.bottom =
                text_rect.top + (text_rect.bottom - text_rect.top) / line_height * line_height;
            SendMessageW(
                child,
                windows::Win32::UI::Controls::EM_SETRECTNP,
                WPARAM(0),
                LPARAM(&text_rect as *const _ as isize),
            );
        }
        InvalidateRect(self.window, None, false);
    }
}

#[cfg(test)]
mod plan_tests {
    use super::*;

    const SIZES: [(i32, i32); 4] = [
        (MIN_WIDTH, MIN_HEIGHT),
        (DEFAULT_WIDTH, DEFAULT_HEIGHT),
        (1280, 800),
        (1920, 1009),
    ];

    fn every_plan() -> Vec<(String, i32, i32, Plan)> {
        let mut plans = Vec::new();
        for (width, height) in SIZES {
            for page in [Page::Triggers, Page::Settings, Page::Credits] {
                for listening in [false, true] {
                    plans.push((
                        format!("{} {width}x{height} {listening}", page.id()),
                        width,
                        height,
                        plan(page, DiagnosticTab::Activity, listening, width, height),
                    ));
                }
            }
            for tab in [DiagnosticTab::Activity, DiagnosticTab::Checks] {
                plans.push((
                    format!("diagnostics {} {width}x{height}", tab.id()),
                    width,
                    height,
                    plan(Page::Diagnostics, tab, false, width, height),
                ));
            }
        }
        plans
    }

    #[test]
    fn every_page_fits_its_window_without_overlap() {
        for (name, width, height, plan) in every_plan() {
            let client = Rect::new(0, 0, width, height);
            for (id, rect) in &plan.items {
                assert!(rect.w > 0 && rect.h > 0, "{name}: control {id} is empty");
                assert!(
                    client.contains(*rect),
                    "{name}: control {id} leaves the window"
                );
                let sidebar = rect.right() <= SIDEBAR;
                assert!(
                    sidebar || rect.x >= LEFT,
                    "{name}: control {id} crosses the sidebar"
                );
            }
            for (index, (first, a)) in plan.items.iter().enumerate() {
                for (second, b) in &plan.items[index + 1..] {
                    assert!(
                        !a.overlaps(*b),
                        "{name}: controls {first} and {second} overlap"
                    );
                }
            }
            let content = Rect::new(LEFT, TOP, width - LEFT - MARGIN, height - FOOTER - 20 - TOP);
            for card in plan.cards.iter().chain(&plan.tracks) {
                assert!(
                    content.contains(*card),
                    "{name}: card {card:?} leaves the content area"
                );
            }
            for (id, rect) in &plan.items {
                for card in &plan.cards {
                    assert!(
                        !card.overlaps(*rect) || card.contains(*rect),
                        "{name}: control {id} straddles card {card:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn shared_controls_stay_available_on_every_page() {
        for (name, _, _, plan) in every_plan() {
            for id in [
                NAV_TRIGGERS,
                NAV_CREDITS,
                SAVE,
                HIDE,
                STATUS,
                MODE,
                PAGE_TITLE,
            ] {
                assert!(
                    plan.get(id).is_some(),
                    "{name}: missing shared control {id}"
                );
            }
            assert!(
                plan.get(START).is_some() != plan.get(STOP).is_some(),
                "{name}: exactly one listening control is shown"
            );
        }
    }

    #[test]
    fn license_paragraphs_flow_with_the_panel() {
        let text = reflow("MIT License\n\nCopyright (c) 2026\n\nPermission is\nhereby granted\n");
        assert_eq!(
            text,
            "MIT License\r\n\r\nCopyright (c) 2026\r\n\r\nPermission is hereby granted"
        );
    }
}
