use super::*;
use windows::Win32::UI::Controls::{SetWindowTheme, PBM_SETBARCOLOR, PBM_SETBKCOLOR};

pub(super) fn cards(
    page: Page,
    tab: DiagnosticTab,
    width: i32,
    height: i32,
) -> Vec<(i32, i32, i32, i32)> {
    let w = width - 272;
    let h = height - 256;
    let list = (w * 30 / 100).clamp(210, 280);
    match page {
        Page::Triggers => vec![(240, 128, list, h), (260 + list, 128, w - list - 20, h)],
        Page::Settings => vec![(240, 128, w, 180), (240, 328, w, h - 200)],
        Page::Diagnostics if tab == DiagnosticTab::Activity => {
            vec![(240, 180, w, 128), (240, 328, w, h - 200)]
        }
        Page::Diagnostics => vec![(240, 180, w, h - 52)],
        Page::Credits => vec![(240, 128, 260, h), (520, 128, w - 280, h)],
    }
}

impl App {
    unsafe fn control(
        &mut self,
        id: u16,
        class: PCWSTR,
        text: &str,
        style: WINDOW_STYLE,
        scope: Scope,
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
        let _ = SetWindowTheme(handle, w!("Explorer"), PCWSTR::null());
        self.controls.push(Control {
            id,
            handle,
            scope,
            font,
        });
        handle
    }

    unsafe fn label(&mut self, id: u16, text: &str, scope: Scope, font: Font) {
        self.control(id, w!("STATIC"), text, WINDOW_STYLE(0), scope, font);
    }

    unsafe fn edit(&mut self, id: u16, text: &str, scope: Scope, multiline: bool) {
        let style = if multiline {
            WS_VSCROLL
                | WINDOW_STYLE(ES_MULTILINE as u32 | ES_READONLY as u32 | ES_AUTOVSCROLL as u32)
        } else {
            WS_BORDER | WINDOW_STYLE(ES_AUTOHSCROLL as u32)
        };
        self.control(
            id,
            w!("EDIT"),
            text,
            WS_TABSTOP | style,
            scope,
            if multiline { Font::Mono } else { Font::Body },
        );
    }

    unsafe fn button(&mut self, id: u16, text: &str, scope: Scope) {
        self.control(
            id,
            w!("BUTTON"),
            text,
            WS_TABSTOP | WINDOW_STYLE(BS_OWNERDRAW as u32),
            scope,
            Font::Body,
        );
    }

    pub(super) unsafe fn initialize(&mut self) {
        use Scope::*;
        self.label(BRAND, "VoiceTrigger", Global, Font::Brand);
        self.label(BRAND_HINT, "Local voice automation", Global, Font::Small);
        self.label(
            VERSION,
            concat!("Version ", env!("CARGO_PKG_VERSION")),
            Global,
            Font::Small,
        );
        self.label(MODE, "Not listening", Global, Font::Small);
        self.label(PAGE_TITLE, "Triggers", Global, Font::Heading);
        self.label(
            PAGE_HINT,
            "Choose a phrase and the action it runs.",
            Global,
            Font::Body,
        );
        for (id, text) in [
            (NAV_TRIGGERS, "Triggers"),
            (NAV_SETTINGS, "Settings"),
            (NAV_DIAGNOSTICS, "Diagnostics"),
            (NAV_CREDITS, "Credits"),
        ] {
            self.button(id, text, Global);
        }
        self.label(
            STATUS,
            "Ready. Start listening to enable your voice triggers.",
            Global,
            Font::Small,
        );
        for (id, text) in [
            (SAVE, "Save configuration"),
            (HIDE, "Hide to tray"),
            (STOP, "Stop"),
            (START, "Start listening"),
        ] {
            self.button(id, text, Global);
        }

        let triggers = Section(Page::Triggers);
        self.label(LIST_TITLE, "Configured triggers", triggers, Font::Label);
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
            triggers,
            Font::Body,
        );
        self.label(EDITOR_TITLE, "Trigger details", triggers, Font::Label);
        self.label(PHRASE_LABEL, "Spoken phrase", triggers, Font::Body);
        self.edit(PHRASE, "", triggers, false);
        self.label(ACTION_LABEL, "Action", triggers, Font::Body);
        self.control(
            ACTION,
            w!("COMBOBOX"),
            "",
            WS_TABSTOP | WS_VSCROLL | WINDOW_STYLE(CBS_DROPDOWNLIST as u32),
            triggers,
            Font::Body,
        );
        for text in [
            "Dictate to ChatGPT",
            "Open ChatGPT",
            "Open a website",
            "Launch an application",
        ] {
            SendMessageW(
                self.child(ACTION),
                CB_ADDSTRING,
                WPARAM(0),
                LPARAM(wide(text).as_ptr() as isize),
            );
        }
        self.label(TARGET_LABEL, "Target", triggers, Font::Body);
        self.edit(TARGET, "", triggers, false);
        self.label(
            TARGET_HINT,
            "Used for a website address or an executable path.",
            triggers,
            Font::Small,
        );
        for (id, text) in [
            (APPLY, "Apply changes"),
            (ADD, "Add as new"),
            (DELETE, "Remove"),
        ] {
            self.button(id, text, triggers);
        }

        let settings = Section(Page::Settings);
        self.label(RECOGNITION_TITLE, "Recognition", settings, Font::Label);
        self.label(LANGUAGE_LABEL, "Recognition language", settings, Font::Body);
        self.control(
            LANGUAGE,
            w!("COMBOBOX"),
            "",
            WS_TABSTOP | WS_VSCROLL | WINDOW_STYLE(CBS_DROPDOWNLIST as u32),
            settings,
            Font::Body,
        );
        for text in ["French (France)", "English (United States)"] {
            SendMessageW(
                self.child(LANGUAGE),
                CB_ADDSTRING,
                WPARAM(0),
                LPARAM(wide(text).as_ptr() as isize),
            );
        }
        SendMessageW(
            self.child(LANGUAGE),
            CB_SETCURSEL,
            WPARAM(usize::from(self.settings.language == 0x0409)),
            LPARAM(0),
        );
        self.label(
            LANGUAGE_HINT,
            "Uses an installed Windows SAPI engine.",
            settings,
            Font::Small,
        );
        self.label(THRESHOLD_LABEL, "Silence threshold", settings, Font::Body);
        self.edit(
            THRESHOLD,
            &self.settings.voice_threshold.to_string(),
            settings,
            false,
        );
        self.label(
            THRESHOLD_HINT,
            "0.001 to 0.5. Default: 0.015.",
            settings,
            Font::Small,
        );
        self.label(CHATGPT_TITLE, "ChatGPT", settings, Font::Label);
        self.label(
            PATH_LABEL,
            "Executable path (optional)",
            settings,
            Font::Body,
        );
        self.edit(
            CHAT_PATH,
            &self.settings.chatgpt_path.clone(),
            settings,
            false,
        );
        self.label(
            PATH_HINT,
            "Leave blank to use the installed app. Dictation submits after 3 seconds of silence.",
            settings,
            Font::Small,
        );

        self.button(ACTIVITY_TAB, "Activity", Section(Page::Diagnostics));
        self.button(CHECKS_TAB, "System checks", Section(Page::Diagnostics));
        let activity = Diagnostic(DiagnosticTab::Activity);
        self.label(INPUT_TITLE, "Microphone", activity, Font::Label);
        self.label(LEVEL, "Microphone: stopped", activity, Font::Body);
        self.control(
            DEBUG,
            w!("BUTTON"),
            "Live debug",
            WS_TABSTOP | WINDOW_STYLE(BS_AUTOCHECKBOX as u32),
            activity,
            Font::Body,
        );
        SendMessageW(
            self.child(DEBUG),
            BM_SETCHECK,
            WPARAM(BST_CHECKED.0 as usize),
            LPARAM(0),
        );
        self.control(
            METER,
            w!("msctls_progress32"),
            "Microphone level",
            WINDOW_STYLE(0),
            activity,
            Font::Body,
        );
        let _ = SetWindowTheme(self.child(METER), w!(""), w!(""));
        SendMessageW(self.child(METER), PBM_SETRANGE32, WPARAM(0), LPARAM(100));
        SendMessageW(
            self.child(METER),
            PBM_SETBARCOLOR,
            WPARAM(0),
            LPARAM(0x677011),
        );
        SendMessageW(
            self.child(METER),
            PBM_SETBKCOLOR,
            WPARAM(0),
            LPARAM(0xf0e9e4),
        );
        self.label(LOG_TITLE, "Event log", activity, Font::Label);
        self.button(CLEAR_LOG, "Clear log", activity);
        self.edit(LOG, "", activity, true);
        let checks = Diagnostic(DiagnosticTab::Checks);
        self.label(CHECKS_TITLE, "System checks", checks, Font::Label);
        self.label(
            CHECKS_HINT,
            "Check installed recognition engines and accessible ChatGPT controls.",
            checks,
            Font::Small,
        );
        self.button(DIAGNOSTIC, "Run checks", checks);
        self.edit(
            REPORT,
            "Run checks to inspect the recognition engine and ChatGPT connection.",
            checks,
            true,
        );

        let credits = Section(Page::Credits);
        self.label(CREDIT_NAME, "VoiceTrigger", credits, Font::Brand);
        self.label(
            CREDIT_VERSION,
            concat!("Version ", env!("CARGO_PKG_VERSION")),
            credits,
            Font::Body,
        );
        self.label(
            CREDIT_OWNER,
            "Created by TheGeogeo\r\nCopyright 2026 TheGeogeo",
            credits,
            Font::Body,
        );
        self.button(REPOSITORY, "Source code", credits);
        self.button(DOCUMENTATION, "Documentation", credits);
        self.label(CREDIT_BUILT, "Built with", credits, Font::Label);
        self.label(CREDIT_LIBRARIES, "Rust\r\nwindows-rs 0.48.0\r\nSerde 1.0.188\r\nserde_json 1.0.105\r\nLibraries: MIT or Apache 2.0", credits, Font::Small);
        self.label(LICENSE_TITLE, "MIT license", credits, Font::Label);
        self.edit(
            LICENSE_TEXT,
            &include_str!("../../../LICENSE")
                .replace("\r\n", "\n")
                .replace('\n', "\r\n"),
            credits,
            true,
        );
        self.append_log("Ready. Trigger and action events are kept in memory only.");
        self.refresh_list(Some(0));
        self.initialized = true;
        self.update_controls(false);
        self.layout();
    }

    unsafe fn place(&self, id: u16, x: i32, y: i32, width: i32, height: i32) {
        SetWindowPos(
            self.child(id),
            None,
            self.theme.px(x),
            self.theme.px(y),
            self.theme.px(width.max(1)),
            self.theme.px(height.max(1)),
            SWP_NOZORDER | SWP_NOACTIVATE,
        );
    }

    pub(super) unsafe fn layout(&self) {
        if !self.initialized {
            return;
        }
        let mut rect = RECT::default();
        GetClientRect(self.window, &mut rect);
        let width = rect.right * 96 / self.theme.dpi.get() as i32;
        let height = rect.bottom * 96 / self.theme.dpi.get() as i32;
        let w = width - 272;
        let h = height - 256;
        self.place(BRAND, 70, 37, 128, 26);
        self.place(BRAND_HINT, 25, 87, 174, 20);
        self.place(MODE, 43, height - 96, 156, 22);
        self.place(VERSION, 25, height - 48, 174, 20);
        for (index, id) in [NAV_TRIGGERS, NAV_SETTINGS, NAV_DIAGNOSTICS, NAV_CREDITS]
            .iter()
            .enumerate()
        {
            self.place(*id, 16, 140 + index as i32 * 50, 176, 44);
        }
        self.place(PAGE_TITLE, 240, 36, w, 38);
        self.place(PAGE_HINT, 240, 82, w, 24);
        self.place(STATUS, 240, height - 106, w, 38);
        self.place(SAVE, 240, height - 56, 154, 38);
        self.place(HIDE, 406, height - 56, 114, 38);
        self.place(STOP, width - 288, height - 56, 84, 38);
        self.place(START, width - 192, height - 56, 160, 38);

        let list = (w * 30 / 100).clamp(210, 280);
        let editor = 260 + list;
        let inner = w - list - 68;
        self.place(LIST_TITLE, 260, 152, list - 40, 24);
        self.place(LIST, 248, 188, list - 16, h - 72);
        self.place(EDITOR_TITLE, editor + 24, 152, inner, 24);
        for (id, y) in [
            (PHRASE_LABEL, 194),
            (ACTION_LABEL, 270),
            (TARGET_LABEL, 346),
        ] {
            self.place(id, editor + 24, y, inner, 22);
        }
        self.place(PHRASE, editor + 24, 220, inner, 34);
        self.place(ACTION, editor + 24, 296, inner, 180);
        self.place(TARGET, editor + 24, 372, inner, 34);
        self.place(TARGET_HINT, editor + 24, 418, inner, 34);
        self.place(APPLY, editor + 24, height - 184, 126, 36);
        self.place(ADD, editor + 160, height - 184, 108, 36);
        self.place(DELETE, editor + 278, height - 184, 82, 36);

        let col = (w - 72) / 2;
        self.place(RECOGNITION_TITLE, 264, 150, w - 48, 24);
        for (id, x) in [(LANGUAGE_LABEL, 264), (THRESHOLD_LABEL, 288 + col)] {
            self.place(id, x, 194, col, 22);
        }
        self.place(LANGUAGE, 264, 222, col, 160);
        self.place(THRESHOLD, 288 + col, 222, col, 32);
        self.place(LANGUAGE_HINT, 264, 270, col, 26);
        self.place(THRESHOLD_HINT, 288 + col, 270, col, 26);
        self.place(CHATGPT_TITLE, 264, 350, w - 48, 24);
        self.place(PATH_LABEL, 264, 390, w - 48, 22);
        self.place(CHAT_PATH, 264, 418, w - 48, 34);
        self.place(PATH_HINT, 264, 466, w - 48, 34);

        self.place(ACTIVITY_TAB, 240, 128, 104, 36);
        self.place(CHECKS_TAB, 356, 128, 140, 36);
        self.place(INPUT_TITLE, 264, 202, w - 48, 24);
        self.place(LEVEL, 264, 245, w - 220, 24);
        self.place(DEBUG, width - 188, 242, 132, 28);
        self.place(METER, 264, 286, w - 48, 8);
        self.place(LOG_TITLE, 264, 350, w - 200, 24);
        self.place(CLEAR_LOG, width - 156, 342, 100, 36);
        self.place(LOG, 264, 390, w - 48, height - 542);
        self.place(CHECKS_TITLE, 264, 204, w - 220, 24);
        self.place(CHECKS_HINT, 264, 242, w - 48, 24);
        self.place(DIAGNOSTIC, width - 180, 198, 124, 36);
        self.place(REPORT, 264, 286, w - 48, height - 438);

        self.place(CREDIT_NAME, 264, 152, 212, 28);
        self.place(CREDIT_VERSION, 264, 192, 212, 24);
        self.place(CREDIT_OWNER, 264, 218, 212, 44);
        self.place(REPOSITORY, 264, 276, 212, 34);
        self.place(DOCUMENTATION, 264, 322, 212, 34);
        self.place(CREDIT_BUILT, 264, 378, 212, 24);
        self.place(CREDIT_LIBRARIES, 264, 412, 212, 88);
        self.place(LICENSE_TITLE, 544, 152, w - 328, 24);
        self.place(LICENSE_TEXT, 544, 198, w - 328, height - 350);
        // Keep scrollable text aligned to complete font lines at every size.
        for id in [LOG, REPORT, LICENSE_TEXT] {
            let child = self.child(id);
            let dc = GetDC(child);
            let old_font = SelectObject(dc, self.theme.font(Font::Mono));
            let mut metrics = TEXTMETRICW::default();
            GetTextMetricsW(dc, &mut metrics);
            SelectObject(dc, old_font);
            ReleaseDC(child, dc);
            let mut text_rect = RECT::default();
            GetClientRect(child, &mut text_rect);
            let padding = self.theme.px(2);
            InflateRect(&mut text_rect, -padding, -padding);
            let line_height = metrics.tmHeight.max(1);
            text_rect.bottom =
                text_rect.top + (text_rect.bottom - text_rect.top) / line_height * line_height;
            SendMessageW(
                child,
                windows::Win32::UI::Controls::EM_SETRECTNP,
                WPARAM(0),
                LPARAM(&text_rect as *const _ as isize),
            );
        }
        for control in &self.controls {
            let visible = match control.scope {
                Scope::Global => true,
                Scope::Section(page) => page == self.view.page.get(),
                Scope::Diagnostic(tab) => {
                    self.view.page.get() == Page::Diagnostics
                        && tab == self.view.diagnostic_tab.get()
                }
            };
            ShowWindow(control.handle, if visible { SW_SHOWNA } else { SW_HIDE });
        }
        InvalidateRect(self.window, None, false);
    }
}
