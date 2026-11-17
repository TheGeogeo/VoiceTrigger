use super::layout::{self, Rect};
use super::*;
use crate::config::Appearance;
use std::cell::Cell;
use windows::Win32::Graphics::Dwm::{
    DwmSetWindowAttribute, DWMWA_BORDER_COLOR, DWMWA_CAPTION_COLOR, DWMWA_TEXT_COLOR,
    DWMWA_USE_IMMERSIVE_DARK_MODE,
};
use windows::Win32::UI::Controls::{
    DRAWITEMSTRUCT, ODS_COMBOBOXEDIT, ODS_DISABLED, ODS_FOCUS, ODS_NOFOCUSRECT, ODS_SELECTED,
    ODT_BUTTON, ODT_COMBOBOX, ODT_LISTBOX, ODT_STATIC,
};

const fn rgb(r: u32, g: u32, b: u32) -> COLORREF {
    COLORREF(r | (g << 8) | (b << 16))
}

/// Color tokens for one appearance. Every painted surface uses these values.
#[derive(Clone, Copy)]
pub(super) struct Palette {
    pub dark: bool,
    pub background: COLORREF,
    pub surface: COLORREF,
    pub border: COLORREF,
    pub input: COLORREF,
    pub input_border: COLORREF,
    pub input_hover: COLORREF,
    pub disabled_fill: COLORREF,
    pub well: COLORREF,
    pub text: COLORREF,
    pub muted: COLORREF,
    pub disabled: COLORREF,
    pub accent: COLORREF,
    pub accent_hover: COLORREF,
    pub accent_pressed: COLORREF,
    pub on_accent: COLORREF,
    pub selected: COLORREF,
    pub nav_active: COLORREF,
    pub nav_hover: COLORREF,
    pub button: COLORREF,
    pub button_border: COLORREF,
    pub button_hover: COLORREF,
    pub button_pressed: COLORREF,
    pub track: COLORREF,
    pub segment: COLORREF,
    pub danger: COLORREF,
    pub success: COLORREF,
}

impl Palette {
    pub(super) const LIGHT: Self = Self {
        dark: false,
        background: rgb(243, 244, 246),
        surface: rgb(255, 255, 255),
        border: rgb(225, 228, 233),
        input: rgb(255, 255, 255),
        input_border: rgb(205, 210, 218),
        input_hover: rgb(160, 167, 178),
        disabled_fill: rgb(243, 244, 246),
        well: rgb(248, 249, 250),
        text: rgb(23, 28, 36),
        muted: rgb(98, 106, 118),
        disabled: rgb(160, 166, 175),
        accent: rgb(15, 118, 110),
        accent_hover: rgb(13, 104, 97),
        accent_pressed: rgb(17, 94, 89),
        on_accent: rgb(255, 255, 255),
        selected: rgb(226, 241, 239),
        nav_active: rgb(228, 231, 235),
        nav_hover: rgb(235, 237, 240),
        button: rgb(255, 255, 255),
        button_border: rgb(205, 210, 218),
        button_hover: rgb(245, 246, 248),
        button_pressed: rgb(234, 236, 239),
        track: rgb(232, 234, 238),
        segment: rgb(255, 255, 255),
        danger: rgb(180, 35, 24),
        success: rgb(22, 163, 74),
    };

    pub(super) const DARK: Self = Self {
        dark: true,
        background: rgb(28, 29, 32),
        surface: rgb(38, 40, 44),
        border: rgb(52, 55, 61),
        input: rgb(31, 33, 37),
        input_border: rgb(70, 74, 82),
        input_hover: rgb(110, 116, 126),
        disabled_fill: rgb(42, 44, 49),
        well: rgb(31, 33, 37),
        text: rgb(232, 234, 237),
        muted: rgb(156, 162, 170),
        disabled: rgb(98, 104, 112),
        accent: rgb(64, 196, 178),
        accent_hover: rgb(88, 210, 193),
        accent_pressed: rgb(52, 170, 154),
        on_accent: rgb(8, 32, 29),
        selected: rgb(33, 64, 61),
        nav_active: rgb(46, 49, 54),
        nav_hover: rgb(38, 40, 45),
        button: rgb(46, 49, 54),
        button_border: rgb(62, 66, 73),
        button_hover: rgb(54, 57, 63),
        button_pressed: rgb(60, 64, 71),
        track: rgb(22, 23, 26),
        segment: rgb(56, 60, 66),
        danger: rgb(249, 112, 102),
        success: rgb(74, 222, 128),
    };

    pub(super) fn of(appearance: Appearance) -> Self {
        match appearance {
            Appearance::Light => Self::LIGHT,
            Appearance::Dark => Self::DARK,
        }
    }
}

#[derive(Clone, Copy)]
pub(super) enum Font {
    Body,
    Small,
    Heading,
    Label,
    Mono,
    Brand,
}

#[derive(Clone, Copy)]
pub(super) enum Brush {
    Background,
    Surface,
    Input,
    Disabled,
    Well,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Nav,
    Primary,
    Secondary,
    Danger,
    Segment,
    Toggle,
}

fn kind(id: u16) -> Kind {
    match id {
        NAV_TRIGGERS..=NAV_CREDITS => Kind::Nav,
        START => Kind::Primary,
        DELETE => Kind::Danger,
        ACTIVITY_TAB | CHECKS_TAB | THEME_LIGHT | THEME_DARK => Kind::Segment,
        DEBUG => Kind::Toggle,
        _ => Kind::Secondary,
    }
}

/// Controls painted directly on the window background instead of a card.
fn on_background(id: u16) -> bool {
    matches!(
        id,
        BRAND
            | VERSION
            | MODE
            | PAGE_TITLE
            | PAGE_HINT
            | STATUS
            | APPEARANCE_TITLE
            | RECOGNITION_TITLE
            | CHATGPT_TITLE
            | SAVE
            | HIDE
            | START
            | STOP
    ) || kind(id) == Kind::Nav
}

fn muted(id: u16) -> bool {
    matches!(
        id,
        PAGE_HINT
            | VERSION
            | TARGET_HINT
            | THEME_HINT
            | LANGUAGE_HINT
            | THRESHOLD_HINT
            | PATH_HINT
            | CHECKS_HINT
            | CREDIT_VERSION
            | CREDIT_LIBRARIES
            | MODE
    )
}

pub(super) struct Theme {
    pub(super) dpi: Cell<u32>,
    fonts: [Cell<HFONT>; 6],
    palette: Cell<Palette>,
    brushes: [Cell<HBRUSH>; 5],
    icons: [Cell<HICON>; 2],
}

impl Theme {
    pub(super) unsafe fn new(dpi: u32, appearance: Appearance) -> Self {
        let palette = Palette::of(appearance);
        Self {
            dpi: Cell::new(dpi),
            fonts: Self::fonts(dpi).map(Cell::new),
            palette: Cell::new(palette),
            brushes: Self::brushes(&palette).map(Cell::new),
            icons: Self::icons(dpi).map(Cell::new),
        }
    }

    unsafe fn fonts(dpi: u32) -> [HFONT; 6] {
        std::array::from_fn(|index| {
            let size = [14, 12, 26, 14, 13, 17][index];
            CreateFontW(
                -((size * dpi as i32 + 48) / 96),
                0,
                0,
                0,
                if matches!(index, 2 | 3 | 5) { 600 } else { 400 },
                0,
                0,
                0,
                DEFAULT_CHARSET.0 as u32,
                OUT_DEFAULT_PRECIS.0 as u32,
                CLIP_DEFAULT_PRECIS.0 as u32,
                CLEARTYPE_QUALITY.0 as u32,
                0,
                if index == 4 {
                    w!("Consolas")
                } else {
                    w!("Segoe UI")
                },
            )
        })
    }

    unsafe fn brushes(palette: &Palette) -> [HBRUSH; 5] {
        [
            palette.background,
            palette.surface,
            palette.input,
            palette.disabled_fill,
            palette.well,
        ]
        .map(|color| CreateSolidBrush(color))
    }

    unsafe fn icons(dpi: u32) -> [HICON; 2] {
        [SM_CXSMICON, SM_CXICON].map(|metric| create_icon(GetSystemMetricsForDpi(metric, dpi)))
    }

    /// Switches palette and brushes. Callers repaint the window afterwards.
    pub(super) unsafe fn set_appearance(&self, appearance: Appearance) {
        let palette = Palette::of(appearance);
        self.palette.set(palette);
        for (cell, brush) in self.brushes.iter().zip(Self::brushes(&palette)) {
            DeleteObject(cell.replace(brush));
        }
    }

    /// Replaces DPI-dependent fonts and icons and returns the retired handles,
    /// which stay valid until controls have received their replacements.
    pub(super) unsafe fn change_dpi(&self, dpi: u32) -> ([HFONT; 6], [HICON; 2]) {
        self.dpi.set(dpi);
        let fonts = Self::fonts(dpi);
        let icons = Self::icons(dpi);
        (
            std::array::from_fn(|index| self.fonts[index].replace(fonts[index])),
            std::array::from_fn(|index| self.icons[index].replace(icons[index])),
        )
    }

    pub(super) fn palette(&self) -> Palette {
        self.palette.get()
    }
    pub(super) fn font(&self, font: Font) -> HFONT {
        self.fonts[font as usize].get()
    }
    pub(super) fn brush(&self, brush: Brush) -> HBRUSH {
        self.brushes[brush as usize].get()
    }
    pub(super) fn icon(&self, large: bool) -> HICON {
        self.icons[usize::from(large)].get()
    }
    pub(super) fn px(&self, value: i32) -> i32 {
        (value * self.dpi.get() as i32 + 48) / 96
    }
    pub(super) fn rect(&self, rect: Rect) -> RECT {
        RECT {
            left: self.px(rect.x),
            top: self.px(rect.y),
            right: self.px(rect.right()),
            bottom: self.px(rect.bottom()),
        }
    }

    /// Physical line height of a font at the current DPI.
    pub(super) unsafe fn line_height(&self, font: Font) -> i32 {
        let dc = GetDC(None);
        let old = SelectObject(dc, self.font(font));
        let mut metrics = TEXTMETRICW::default();
        GetTextMetricsW(dc, &mut metrics);
        SelectObject(dc, old);
        ReleaseDC(None, dc);
        metrics.tmHeight.max(1)
    }

    /// Colors the native title bar and border so the window reads as one surface.
    pub(super) unsafe fn apply_frame(&self, window: HWND) {
        let palette = self.palette();
        let dark = BOOL::from(palette.dark);
        let attributes = [
            (
                DWMWA_USE_IMMERSIVE_DARK_MODE,
                &dark as *const _ as *const _,
                std::mem::size_of::<BOOL>(),
            ),
            (
                DWMWA_CAPTION_COLOR,
                &palette.background as *const _ as *const _,
                std::mem::size_of::<COLORREF>(),
            ),
            (
                DWMWA_TEXT_COLOR,
                &palette.text as *const _ as *const _,
                std::mem::size_of::<COLORREF>(),
            ),
            (
                DWMWA_BORDER_COLOR,
                &palette.border as *const _ as *const _,
                std::mem::size_of::<COLORREF>(),
            ),
        ];
        for (attribute, value, size) in attributes {
            // Older Windows versions ignore unsupported caption attributes.
            let _ = DwmSetWindowAttribute(window, attribute, value, size as u32);
        }
    }

    pub(super) unsafe fn control_color(
        &self,
        dc: HDC,
        id: u16,
        enabled: bool,
        edit: bool,
    ) -> HBRUSH {
        let palette = self.palette();
        let (brush, text) = match id {
            LOG | REPORT | LICENSE_TEXT => (Brush::Well, palette.text),
            PHRASE | TARGET | THRESHOLD | CHAT_PATH if !enabled => {
                (Brush::Disabled, palette.disabled)
            }
            _ if edit => (Brush::Input, palette.text),
            _ if on_background(id) => (
                Brush::Background,
                if muted(id) {
                    palette.muted
                } else {
                    palette.text
                },
            ),
            _ => (
                Brush::Surface,
                if !enabled {
                    palette.disabled
                } else if muted(id) {
                    palette.muted
                } else {
                    palette.text
                },
            ),
        };
        SetTextColor(dc, text);
        SetBkColor(dc, self.color(brush));
        SetBkMode(dc, OPAQUE);
        self.brush(brush)
    }

    fn color(&self, brush: Brush) -> COLORREF {
        let palette = self.palette();
        match brush {
            Brush::Background => palette.background,
            Brush::Surface => palette.surface,
            Brush::Input => palette.input,
            Brush::Disabled => palette.disabled_fill,
            Brush::Well => palette.well,
        }
    }

    unsafe fn fill(&self, dc: HDC, rect: &RECT, color: COLORREF) {
        let brush = CreateSolidBrush(color);
        FillRect(dc, rect, brush);
        DeleteObject(brush);
    }

    unsafe fn round(
        &self,
        dc: HDC,
        rect: RECT,
        fill: COLORREF,
        border: COLORREF,
        radius: i32,
        line: i32,
    ) {
        let brush = CreateSolidBrush(fill);
        let pen = CreatePen(PS_INSIDEFRAME, self.px(line).max(1), border);
        let old_brush = SelectObject(dc, brush);
        let old_pen = SelectObject(dc, pen);
        RoundRect(
            dc,
            rect.left,
            rect.top,
            rect.right,
            rect.bottom,
            self.px(radius),
            self.px(radius),
        );
        SelectObject(dc, old_pen);
        SelectObject(dc, old_brush);
        DeleteObject(pen);
        DeleteObject(brush);
    }

    unsafe fn text(
        &self,
        dc: HDC,
        text: &str,
        mut rect: RECT,
        color: COLORREF,
        font: Font,
        flags: DRAW_TEXT_FORMAT,
    ) {
        let old = SelectObject(dc, self.font(font));
        SetBkMode(dc, TRANSPARENT);
        SetTextColor(dc, color);
        let mut text: Vec<u16> = text.encode_utf16().collect();
        DrawTextW(dc, &mut text, &mut rect, flags | DT_NOPREFIX);
        SelectObject(dc, old);
    }

    unsafe fn chevron(&self, dc: HDC, x: i32, y: i32, color: COLORREF) {
        let pen = CreatePen(PS_SOLID, self.px(1).max(1), color);
        let old = SelectObject(dc, pen);
        let size = self.px(4);
        MoveToEx(dc, x - size, y - size / 2, None);
        LineTo(dc, x, y + size / 2);
        LineTo(dc, x + size + 1, y - size / 2 - 1);
        SelectObject(dc, old);
        DeleteObject(pen);
    }

    /// Paints the window background, cards, field frames and status indicator
    /// through an off-screen buffer to avoid resize flicker.
    pub(super) unsafe fn paint(&self, window: HWND, view: &ViewState) {
        let mut paint = PAINTSTRUCT::default();
        let target = BeginPaint(window, &mut paint);
        let mut client = RECT::default();
        GetClientRect(window, &mut client);
        let memory = CreateCompatibleDC(target);
        let dc = HDC(memory.0);
        let bitmap = CreateCompatibleBitmap(target, client.right.max(1), client.bottom.max(1));
        let old_bitmap = SelectObject(dc, bitmap);
        let palette = self.palette();
        self.fill(dc, &client, palette.background);
        let dpi = self.dpi.get() as i32;
        let (width, height) = (client.right * 96 / dpi, client.bottom * 96 / dpi);
        let plan = layout::plan(
            view.page.get(),
            view.diagnostic_tab.get(),
            view.listening.get(),
            width,
            height,
        );
        let icon = self.rect(layout::BRAND_ICON);
        DrawIconEx(
            dc,
            icon.left,
            icon.top,
            self.icon(true),
            icon.right - icon.left,
            icon.bottom - icon.top,
            0,
            None,
            DI_NORMAL,
        );
        for card in &plan.cards {
            self.round(dc, self.rect(*card), palette.surface, palette.border, 10, 1);
        }
        for track in &plan.tracks {
            self.round(dc, self.rect(*track), palette.track, palette.track, 8, 1);
        }
        for divider in &plan.dividers {
            self.fill(dc, &self.rect(*divider), palette.border);
        }
        let focus = GetFocus();
        for (id, frame) in &plan.items {
            let child = GetDlgItem(window, *id as i32);
            match *id {
                PHRASE | TARGET | THRESHOLD | CHAT_PATH => {
                    let enabled = IsWindowEnabled(child).as_bool();
                    let focused = focus == child;
                    self.round(
                        dc,
                        self.rect(*frame),
                        if enabled {
                            palette.input
                        } else {
                            palette.disabled_fill
                        },
                        if focused {
                            palette.accent
                        } else if enabled {
                            palette.input_border
                        } else {
                            palette.border
                        },
                        6,
                        if focused { 2 } else { 1 },
                    );
                }
                LOG | REPORT | LICENSE_TEXT => {
                    self.round(dc, self.rect(*frame), palette.well, palette.border, 6, 1);
                }
                _ => {}
            }
        }
        let dot = if view.listening.get() {
            palette.success
        } else {
            palette.disabled
        };
        self.round(dc, self.rect(layout::status_dot(height)), dot, dot, 8, 1);
        BitBlt(target, 0, 0, client.right, client.bottom, dc, 0, 0, SRCCOPY);
        SelectObject(dc, old_bitmap);
        DeleteObject(bitmap);
        DeleteDC(memory);
        EndPaint(window, &paint);
    }

    /// Paints a closed select with the same frame as text fields.
    pub(super) unsafe fn paint_select(&self, select: HWND, target: HDC, hover: bool) {
        let mut client = RECT::default();
        GetClientRect(select, &mut client);
        let memory = CreateCompatibleDC(target);
        let dc = HDC(memory.0);
        let bitmap = CreateCompatibleBitmap(target, client.right.max(1), client.bottom.max(1));
        let old_bitmap = SelectObject(dc, bitmap);
        let palette = self.palette();
        let enabled = IsWindowEnabled(select).as_bool();
        let open = SendMessageW(select, CB_GETDROPPEDSTATE, WPARAM(0), LPARAM(0)).0 != 0;
        let focused = GetFocus() == select || open;
        self.fill(dc, &client, palette.surface);
        self.round(
            dc,
            client,
            if enabled {
                palette.input
            } else {
                palette.disabled_fill
            },
            if focused {
                palette.accent
            } else if !enabled {
                palette.border
            } else if hover {
                palette.input_hover
            } else {
                palette.input_border
            },
            6,
            if focused { 2 } else { 1 },
        );
        let index = SendMessageW(select, CB_GETCURSEL, WPARAM(0), LPARAM(0)).0;
        if index >= 0 {
            let length = SendMessageW(select, CB_GETLBTEXTLEN, WPARAM(index as usize), LPARAM(0)).0;
            if length > 0 {
                let mut text = vec![0u16; length as usize + 1];
                SendMessageW(
                    select,
                    CB_GETLBTEXT,
                    WPARAM(index as usize),
                    LPARAM(text.as_mut_ptr() as isize),
                );
                let mut rect = client;
                rect.left += self.px(12);
                rect.right -= self.px(36);
                self.text(
                    dc,
                    &String::from_utf16_lossy(&text[..length as usize]),
                    rect,
                    if enabled {
                        palette.text
                    } else {
                        palette.disabled
                    },
                    Font::Body,
                    DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS,
                );
            }
        }
        self.chevron(
            dc,
            client.right - self.px(18),
            (client.top + client.bottom) / 2,
            if enabled {
                palette.muted
            } else {
                palette.disabled
            },
        );
        BitBlt(target, 0, 0, client.right, client.bottom, dc, 0, 0, SRCCOPY);
        SelectObject(dc, old_bitmap);
        DeleteObject(bitmap);
        DeleteDC(memory);
    }

    pub(super) unsafe fn draw_item(&self, item: &DRAWITEMSTRUCT, view: &ViewState) {
        match item.CtlType {
            ODT_LISTBOX => self.draw_row(item),
            ODT_COMBOBOX => self.draw_option(item),
            ODT_STATIC => self.draw_static(item, view),
            ODT_BUTTON => self.draw_button(item, view),
            _ => {}
        }
    }

    unsafe fn item_text(window: HWND) -> String {
        let mut text = vec![0u16; GetWindowTextLengthW(window) as usize + 1];
        let count = GetWindowTextW(window, &mut text);
        String::from_utf16_lossy(&text[..count as usize])
    }

    unsafe fn draw_button(&self, item: &DRAWITEMSTRUCT, view: &ViewState) {
        let palette = self.palette();
        let id = item.CtlID as u16;
        let kind = kind(id);
        let state = item.itemState.0;
        let disabled = state & ODS_DISABLED.0 != 0;
        let pressed = state & ODS_SELECTED.0 != 0;
        let hover = view.hover.get() == item.hwndItem.0 && !disabled;
        let focus = state & ODS_FOCUS.0 != 0 && state & ODS_NOFOCUSRECT.0 == 0;
        let active = match id {
            NAV_TRIGGERS..=NAV_CREDITS => id == view.page.get().id(),
            ACTIVITY_TAB | CHECKS_TAB => id == view.diagnostic_tab.get().id(),
            THEME_LIGHT => !palette.dark,
            THEME_DARK => palette.dark,
            _ => false,
        };
        let rect = item.rcItem;
        let dc = item.hDC;
        let parent = if kind == Kind::Segment {
            palette.track
        } else if on_background(id) {
            palette.background
        } else {
            palette.surface
        };
        self.fill(dc, &rect, parent);
        let text = Self::item_text(item.hwndItem);
        let mut label = rect;
        label.left += self.px(10);
        label.right -= self.px(10);
        let mut font = Font::Body;
        let color = match kind {
            Kind::Nav => {
                if active || pressed || hover {
                    let fill = if active {
                        palette.nav_active
                    } else {
                        palette.nav_hover
                    };
                    self.round(dc, rect, fill, fill, 8, 1);
                }
                if active {
                    font = Font::Label;
                    let mut bar = rect;
                    bar.left += self.px(4);
                    bar.right = bar.left + self.px(3);
                    bar.top += self.px(12);
                    bar.bottom -= self.px(12);
                    self.round(dc, bar, palette.accent, palette.accent, 3, 1);
                }
                palette.text
            }
            Kind::Primary => {
                let fill = if disabled {
                    palette.disabled_fill
                } else if pressed {
                    palette.accent_pressed
                } else if hover {
                    palette.accent_hover
                } else {
                    palette.accent
                };
                self.round(dc, rect, fill, fill, 8, 1);
                font = Font::Label;
                if disabled {
                    palette.disabled
                } else {
                    palette.on_accent
                }
            }
            Kind::Secondary | Kind::Danger => {
                let fill = if disabled {
                    palette.disabled_fill
                } else if pressed {
                    palette.button_pressed
                } else if hover {
                    palette.button_hover
                } else {
                    palette.button
                };
                self.round(dc, rect, fill, palette.button_border, 8, 1);
                if disabled {
                    palette.disabled
                } else if kind == Kind::Danger {
                    palette.danger
                } else {
                    palette.text
                }
            }
            Kind::Segment => {
                if active {
                    self.round(dc, rect, palette.segment, palette.border, 6, 1);
                    font = Font::Label;
                    palette.text
                } else if hover || pressed {
                    palette.text
                } else {
                    palette.muted
                }
            }
            Kind::Toggle => {
                let on = view.live_debug.get();
                let switch_width = self.px(40);
                let switch_height = self.px(22);
                let mut switch = rect;
                switch.right -= self.px(2);
                switch.left = switch.right - switch_width;
                switch.top += (rect.bottom - rect.top - switch_height) / 2;
                switch.bottom = switch.top + switch_height;
                let track = if on {
                    if hover {
                        palette.accent_hover
                    } else {
                        palette.accent
                    }
                } else {
                    palette.track
                };
                self.round(
                    dc,
                    switch,
                    track,
                    if on { track } else { palette.input_border },
                    22,
                    1,
                );
                let knob = self.px(14);
                let inset = (switch_height - knob) / 2;
                let left = if on {
                    switch.right - inset - knob
                } else {
                    switch.left + inset
                };
                let knob_rect = RECT {
                    left,
                    top: switch.top + inset,
                    right: left + knob,
                    bottom: switch.top + inset + knob,
                };
                let knob_color = if on { palette.on_accent } else { palette.muted };
                self.round(dc, knob_rect, knob_color, knob_color, 14, 1);
                label.left = rect.left;
                label.right = switch.left - self.px(10);
                self.text(
                    dc,
                    &text,
                    label,
                    palette.text,
                    Font::Body,
                    DT_SINGLELINE | DT_VCENTER | DT_RIGHT | DT_END_ELLIPSIS,
                );
                if focus {
                    let mut ring = switch;
                    InflateRect(&mut ring, self.px(3), self.px(3));
                    self.outline(dc, ring, palette.accent, 25);
                }
                return;
            }
        };
        self.text(
            dc,
            &text,
            label,
            color,
            font,
            DT_SINGLELINE | DT_VCENTER | DT_CENTER | DT_END_ELLIPSIS,
        );
        if focus {
            let mut ring = rect;
            InflateRect(&mut ring, -self.px(2), -self.px(2));
            let color = if kind == Kind::Primary {
                palette.on_accent
            } else {
                palette.accent
            };
            self.outline(dc, ring, color, 6);
        }
    }

    unsafe fn outline(&self, dc: HDC, rect: RECT, color: COLORREF, radius: i32) {
        let pen = CreatePen(PS_INSIDEFRAME, self.px(2).max(1), color);
        let old_pen = SelectObject(dc, pen);
        let old_brush = SelectObject(dc, GetStockObject(NULL_BRUSH));
        RoundRect(
            dc,
            rect.left,
            rect.top,
            rect.right,
            rect.bottom,
            self.px(radius),
            self.px(radius),
        );
        SelectObject(dc, old_brush);
        SelectObject(dc, old_pen);
        DeleteObject(pen);
    }

    unsafe fn draw_static(&self, item: &DRAWITEMSTRUCT, view: &ViewState) {
        let palette = self.palette();
        let rect = item.rcItem;
        if item.CtlID as u16 == METER {
            self.fill(item.hDC, &rect, palette.surface);
            let height = rect.bottom - rect.top;
            self.round(item.hDC, rect, palette.track, palette.track, height, 1);
            let level = i32::from(view.level.get().min(100));
            if level > 0 {
                let mut bar = rect;
                bar.right = bar.left + ((rect.right - rect.left) * level / 100).max(height);
                self.round(item.hDC, bar, palette.accent, palette.accent, height, 1);
            }
            return;
        }
        // Status text: wrapped and vertically centered beside the footer buttons.
        self.fill(item.hDC, &rect, palette.background);
        let text = Self::item_text(item.hwndItem);
        let mut measure = rect;
        let old = SelectObject(item.hDC, self.font(Font::Small));
        let mut buffer: Vec<u16> = text.encode_utf16().collect();
        DrawTextW(
            item.hDC,
            &mut buffer,
            &mut measure,
            DT_CALCRECT | DT_WORDBREAK | DT_NOPREFIX | DT_EDITCONTROL,
        );
        SelectObject(item.hDC, old);
        let mut target = rect;
        let used = measure.bottom - measure.top;
        if used < rect.bottom - rect.top {
            target.top += (rect.bottom - rect.top - used) / 2;
        }
        self.text(
            item.hDC,
            &text,
            target,
            if text.starts_with("Error") {
                palette.danger
            } else {
                palette.muted
            },
            Font::Small,
            DT_WORDBREAK | DT_EDITCONTROL | DT_END_ELLIPSIS,
        );
    }

    unsafe fn draw_option(&self, item: &DRAWITEMSTRUCT) {
        if item.itemState.0 & ODS_COMBOBOXEDIT.0 != 0 {
            // The closed select is painted by its subclass; refresh it instead.
            InvalidateRect(item.hwndItem, None, false);
            return;
        }
        if item.itemID == u32::MAX {
            return;
        }
        let palette = self.palette();
        let selected = item.itemState.0 & ODS_SELECTED.0 != 0;
        self.fill(item.hDC, &item.rcItem, palette.input);
        if selected {
            let mut pill = item.rcItem;
            InflateRect(&mut pill, -self.px(3), -self.px(2));
            self.round(item.hDC, pill, palette.selected, palette.selected, 6, 1);
        }
        let length = SendMessageW(
            item.hwndItem,
            CB_GETLBTEXTLEN,
            WPARAM(item.itemID as usize),
            LPARAM(0),
        )
        .0;
        if length <= 0 {
            return;
        }
        let mut text = vec![0u16; length as usize + 1];
        SendMessageW(
            item.hwndItem,
            CB_GETLBTEXT,
            WPARAM(item.itemID as usize),
            LPARAM(text.as_mut_ptr() as isize),
        );
        let mut rect = item.rcItem;
        rect.left += self.px(12);
        rect.right -= self.px(12);
        self.text(
            item.hDC,
            &String::from_utf16_lossy(&text[..length as usize]),
            rect,
            palette.text,
            Font::Body,
            DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS,
        );
    }

    unsafe fn draw_row(&self, item: &DRAWITEMSTRUCT) {
        if item.itemID == u32::MAX {
            return;
        }
        let length = SendMessageW(
            item.hwndItem,
            LB_GETTEXTLEN,
            WPARAM(item.itemID as usize),
            LPARAM(0),
        )
        .0;
        if length < 0 {
            return;
        }
        let mut text = vec![0u16; length as usize + 1];
        SendMessageW(
            item.hwndItem,
            LB_GETTEXT,
            WPARAM(item.itemID as usize),
            LPARAM(text.as_mut_ptr() as isize),
        );
        let text = String::from_utf16_lossy(&text[..length as usize]);
        let (phrase, action) = text.split_once('\t').unwrap_or((&text, ""));
        let palette = self.palette();
        let state = item.itemState.0;
        let selected = state & ODS_SELECTED.0 != 0;
        self.fill(item.hDC, &item.rcItem, palette.surface);
        let mut pill = item.rcItem;
        InflateRect(&mut pill, -self.px(4), -self.px(2));
        if selected {
            self.round(item.hDC, pill, palette.selected, palette.selected, 8, 1);
        }
        if state & ODS_FOCUS.0 != 0 && state & ODS_NOFOCUSRECT.0 == 0 {
            self.outline(item.hDC, pill, palette.accent, 8);
        }
        let mut rect = pill;
        rect.left += self.px(12);
        rect.right -= self.px(12);
        rect.top += self.px(6);
        rect.bottom = rect.top + self.px(22);
        self.text(
            item.hDC,
            phrase,
            rect,
            palette.text,
            Font::Label,
            DT_SINGLELINE | DT_VCENTER | DT_WORD_ELLIPSIS,
        );
        rect.top = rect.bottom;
        rect.bottom = rect.top + self.px(18);
        self.text(
            item.hDC,
            action,
            rect,
            palette.muted,
            Font::Small,
            DT_SINGLELINE | DT_VCENTER | DT_WORD_ELLIPSIS,
        );
    }
}

impl Drop for Theme {
    fn drop(&mut self) {
        unsafe {
            for font in &self.fonts {
                DeleteObject(font.get());
            }
            for brush in &self.brushes {
                DeleteObject(brush.get());
            }
            for icon in &self.icons {
                DestroyIcon(icon.get());
            }
        }
    }
}

fn inside_rounded(x: f32, y: f32, center: (f32, f32), half: (f32, f32), radius: f32) -> bool {
    let dx = ((x - center.0).abs() - (half.0 - radius)).max(0.0);
    let dy = ((y - center.1).abs() - (half.1 - radius)).max(0.0);
    dx * dx + dy * dy <= radius * radius
}

fn microphone(x: f32, y: f32) -> bool {
    let distance = ((x - 0.5).powi(2) + (y - 0.46).powi(2)).sqrt();
    inside_rounded(x, y, (0.5, 0.39), (0.12, 0.2), 0.12)
        || (y >= 0.46 && (0.19..=0.255).contains(&distance))
        || ((x - 0.5).abs() <= 0.032 && (0.7..=0.8).contains(&y))
        || inside_rounded(x, y, (0.5, 0.8), (0.13, 0.032), 0.032)
}

/// Anti-aliased application icon pixels (0xAARRGGBB, top-down): a white
/// microphone on a rounded accent square, rendered with 4x4 supersampling.
pub(super) fn icon_pixels(size: i32) -> Vec<u32> {
    const SAMPLES: i32 = 4;
    let (red, green, blue) = (15.0, 118.0, 110.0);
    let mut pixels = Vec::with_capacity((size * size) as usize);
    for row in 0..size {
        for column in 0..size {
            let (mut shape, mut glyph) = (0.0f32, 0.0f32);
            for sy in 0..SAMPLES {
                for sx in 0..SAMPLES {
                    let x = (column as f32 + (sx as f32 + 0.5) / SAMPLES as f32) / size as f32;
                    let y = (row as f32 + (sy as f32 + 0.5) / SAMPLES as f32) / size as f32;
                    if inside_rounded(x, y, (0.5, 0.5), (0.5, 0.5), 0.22) {
                        shape += 1.0;
                        if microphone(x, y) {
                            glyph += 1.0;
                        }
                    }
                }
            }
            let alpha = shape / (SAMPLES * SAMPLES) as f32;
            let white = if shape > 0.0 { glyph / shape } else { 0.0 };
            let mix = |base: f32| (base + (255.0 - base) * white).round() as u32;
            pixels.push(
                ((alpha * 255.0).round() as u32) << 24
                    | mix(red) << 16
                    | mix(green) << 8
                    | mix(blue),
            );
        }
    }
    pixels
}

unsafe fn create_icon(size: i32) -> HICON {
    let info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: size,
            biHeight: -size,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0 as u32,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut bits = std::ptr::null_mut();
    let Ok(color) = CreateDIBSection(None, &info, DIB_RGB_COLORS, &mut bits, None, 0) else {
        return LoadIconW(None, IDI_APPLICATION).unwrap_or_default();
    };
    let pixels = icon_pixels(size);
    std::ptr::copy_nonoverlapping(pixels.as_ptr(), bits.cast::<u32>(), pixels.len());
    let mask = CreateBitmap(size, size, 1, 1, None);
    let icon = CreateIconIndirect(&ICONINFO {
        fIcon: TRUE,
        xHotspot: 0,
        yHotspot: 0,
        hbmMask: mask,
        hbmColor: color,
    });
    DeleteObject(mask);
    DeleteObject(color);
    icon.unwrap_or_else(|_| LoadIconW(None, IDI_APPLICATION).unwrap_or_default())
}

#[cfg(test)]
mod icon_tests {
    use super::icon_pixels;

    #[test]
    fn icon_has_transparent_corners_and_a_white_microphone() {
        for size in [16, 32, 48] {
            let pixels = icon_pixels(size);
            assert_eq!(pixels.len(), (size * size) as usize);
            assert_eq!(pixels[0] >> 24, 0, "corner must be transparent");
            let center = pixels[(size * 2 / 5 * size + size / 2) as usize];
            assert_eq!(center >> 24, 255, "background must be opaque");
            assert!(center & 0xff > 200, "microphone must be light");
            let edge = pixels[(size / 2 * size + size / 10) as usize];
            assert_eq!(
                edge & 0xff_ffff,
                0x0f_766e,
                "background uses the accent color"
            );
        }
    }
}
