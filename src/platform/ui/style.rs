use super::*;
use std::cell::Cell;
use windows::Win32::UI::Controls::{
    DRAWITEMSTRUCT, ODS_DISABLED, ODS_FOCUS, ODS_SELECTED, ODT_LISTBOX,
};

const fn rgb(r: u32, g: u32, b: u32) -> COLORREF {
    COLORREF(r | (g << 8) | (b << 16))
}

pub(super) const BACKGROUND: COLORREF = rgb(245, 247, 250);
pub(super) const SURFACE: COLORREF = rgb(255, 255, 255);
pub(super) const SIDEBAR: COLORREF = rgb(20, 31, 48);
pub(super) const TEXT: COLORREF = rgb(30, 43, 59);
pub(super) const MUTED: COLORREF = rgb(100, 115, 134);
const LINE: COLORREF = rgb(223, 229, 237);
const ACCENT: COLORREF = rgb(17, 112, 103);
const SELECTED: COLORREF = rgb(230, 244, 241);

#[derive(Clone, Copy)]
pub(super) enum Font {
    Body,
    Small,
    Heading,
    Label,
    Mono,
    Brand,
}

pub(super) struct Theme {
    pub(super) dpi: Cell<u32>,
    fonts: [Cell<HFONT>; 6],
    background: HBRUSH,
    surface: HBRUSH,
    sidebar: HBRUSH,
}

impl Theme {
    pub(super) unsafe fn new(dpi: u32) -> Self {
        Self {
            dpi: Cell::new(dpi),
            fonts: Self::fonts(dpi).map(Cell::new),
            background: CreateSolidBrush(BACKGROUND),
            surface: CreateSolidBrush(SURFACE),
            sidebar: CreateSolidBrush(SIDEBAR),
        }
    }

    unsafe fn fonts(dpi: u32) -> [HFONT; 6] {
        std::array::from_fn(|index| {
            let size = [15, 12, 28, 15, 13, 18][index];
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
                if size == 13 {
                    w!("Consolas")
                } else {
                    w!("Segoe UI")
                },
            )
        })
    }

    pub(super) unsafe fn change_dpi(&self, dpi: u32) -> [HFONT; 6] {
        self.dpi.set(dpi);
        let new = Self::fonts(dpi);
        std::array::from_fn(|index| self.fonts[index].replace(new[index]))
    }

    pub(super) fn font(&self, font: Font) -> HFONT {
        self.fonts[font as usize].get()
    }
    pub(super) fn px(&self, value: i32) -> i32 {
        (value * self.dpi.get() as i32 + 48) / 96
    }

    pub(super) unsafe fn control_color(&self, dc: HDC, id: u16, enabled: bool) -> HBRUSH {
        let sidebar = matches!(id, BRAND | BRAND_HINT | VERSION | MODE);
        let frame = matches!(id, PAGE_TITLE | PAGE_HINT | STATUS);
        SetTextColor(
            dc,
            if !enabled {
                MUTED
            } else if sidebar {
                rgb(207, 218, 231)
            } else if matches!(
                id,
                PAGE_HINT
                    | TARGET_HINT
                    | LANGUAGE_HINT
                    | THRESHOLD_HINT
                    | PATH_HINT
                    | CREDIT_VERSION
                    | CREDIT_LIBRARIES
            ) {
                MUTED
            } else {
                TEXT
            },
        );
        SetBkMode(dc, TRANSPARENT);
        let color = if sidebar {
            SIDEBAR
        } else if frame {
            BACKGROUND
        } else {
            SURFACE
        };
        SetBkColor(dc, color);
        if sidebar {
            self.sidebar
        } else if frame {
            self.background
        } else {
            self.surface
        }
    }

    unsafe fn box_rect(&self, dc: HDC, rect: RECT, fill: COLORREF, border: COLORREF, radius: i32) {
        let brush = CreateSolidBrush(fill);
        let pen = CreatePen(PS_SOLID, 1, border);
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

    pub(super) unsafe fn paint(&self, window: HWND, view: &ViewState) {
        let mut paint = PAINTSTRUCT::default();
        let dc = BeginPaint(window, &mut paint);
        let mut client = RECT::default();
        GetClientRect(window, &mut client);
        FillRect(dc, &client, self.background);
        FillRect(
            dc,
            &RECT {
                right: self.px(208),
                ..client
            },
            self.sidebar,
        );
        let width = client.right * 96 / self.dpi.get() as i32;
        let height = client.bottom * 96 / self.dpi.get() as i32;
        for (x, y, w, h) in layout::cards(view.page.get(), view.diagnostic_tab.get(), width, height)
        {
            self.box_rect(dc, self.rect(x, y, w, h), SURFACE, LINE, 12);
        }
        let line = CreateSolidBrush(LINE);
        FillRect(dc, &self.rect(240, height - 116, width - 272, 1), line);
        DeleteObject(line);
        self.box_rect(dc, self.rect(24, 38, 32, 36), ACCENT, ACCENT, 10);
        let pen = CreatePen(PS_SOLID, self.px(2), SURFACE);
        let old_pen = SelectObject(dc, pen);
        let old_brush = SelectObject(dc, GetStockObject(NULL_BRUSH));
        RoundRect(
            dc,
            self.px(35),
            self.px(44),
            self.px(45),
            self.px(61),
            self.px(10),
            self.px(10),
        );
        MoveToEx(dc, self.px(31), self.px(55), None);
        LineTo(dc, self.px(31), self.px(59));
        LineTo(dc, self.px(40), self.px(66));
        LineTo(dc, self.px(49), self.px(59));
        LineTo(dc, self.px(49), self.px(55));
        MoveToEx(dc, self.px(40), self.px(66), None);
        LineTo(dc, self.px(40), self.px(70));
        SelectObject(dc, old_brush);
        SelectObject(dc, old_pen);
        DeleteObject(pen);
        let dot = if view.listening.get() {
            rgb(75, 202, 161)
        } else {
            MUTED
        };
        self.box_rect(dc, self.rect(26, height - 91, 7, 7), dot, dot, 7);
        EndPaint(window, &paint);
    }

    fn rect(&self, x: i32, y: i32, width: i32, height: i32) -> RECT {
        RECT {
            left: self.px(x),
            top: self.px(y),
            right: self.px(x + width),
            bottom: self.px(y + height),
        }
    }

    pub(super) unsafe fn draw_item(&self, item: &DRAWITEMSTRUCT, view: &ViewState) {
        if item.CtlType == ODT_LISTBOX {
            self.draw_row(item);
            return;
        }
        let id = item.CtlID as u16;
        let nav = (NAV_TRIGGERS..=NAV_CREDITS).contains(&id);
        let tab = matches!(id, ACTIVITY_TAB | CHECKS_TAB);
        let active =
            nav && id == view.page.get().id() || tab && id == view.diagnostic_tab.get().id();
        let disabled = (item.itemState.0 & ODS_DISABLED.0) != 0;
        let pressed = (item.itemState.0 & ODS_SELECTED.0) != 0;
        let primary = id == START;
        let parent = if nav {
            self.sidebar
        } else if tab || matches!(id, SAVE | START | STOP | HIDE) {
            self.background
        } else {
            self.surface
        };
        FillRect(item.hDC, &item.rcItem, parent);
        let (fill, border, color) = if nav {
            (
                if active || pressed {
                    rgb(39, 57, 78)
                } else {
                    SIDEBAR
                },
                SIDEBAR,
                if active { SURFACE } else { rgb(179, 194, 213) },
            )
        } else if disabled {
            (rgb(237, 240, 244), LINE, rgb(151, 161, 175))
        } else if primary {
            (
                if pressed { rgb(12, 88, 81) } else { ACCENT },
                ACCENT,
                SURFACE,
            )
        } else if tab && active {
            (SELECTED, SELECTED, ACCENT)
        } else {
            (
                if pressed { rgb(236, 240, 245) } else { SURFACE },
                if tab { BACKGROUND } else { LINE },
                if id == DELETE { rgb(166, 53, 58) } else { TEXT },
            )
        };
        self.box_rect(item.hDC, item.rcItem, fill, border, 8);
        if nav && active {
            let brush = CreateSolidBrush(rgb(75, 202, 161));
            let mut strip = item.rcItem;
            strip.left += self.px(1);
            strip.right = strip.left + self.px(3);
            strip.top += self.px(11);
            strip.bottom -= self.px(11);
            FillRect(item.hDC, &strip, brush);
            DeleteObject(brush);
        }
        let mut text = vec![0u16; GetWindowTextLengthW(item.hwndItem) as usize + 1];
        let count = GetWindowTextW(item.hwndItem, &mut text);
        let mut rect = item.rcItem;
        rect.left += self.px(8);
        rect.right -= self.px(8);
        self.text(
            item.hDC,
            &String::from_utf16_lossy(&text[..count as usize]),
            rect,
            color,
            Font::Body,
            DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS | DT_CENTER,
        );
        if (item.itemState.0 & ODS_FOCUS.0) != 0 {
            let mut focus = item.rcItem;
            InflateRect(&mut focus, -self.px(4), -self.px(4));
            DrawFocusRect(item.hDC, &focus);
        }
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
        let selected = (item.itemState.0 & ODS_SELECTED.0) != 0;
        let brush = CreateSolidBrush(if selected { SELECTED } else { SURFACE });
        FillRect(item.hDC, &item.rcItem, brush);
        DeleteObject(brush);
        let mut rect = item.rcItem;
        rect.left += self.px(14);
        rect.right -= self.px(12);
        rect.top += self.px(12);
        rect.bottom = rect.top + self.px(22);
        self.text(
            item.hDC,
            phrase,
            rect,
            TEXT,
            Font::Label,
            DT_SINGLELINE | DT_WORD_ELLIPSIS,
        );
        rect.top += self.px(25);
        rect.bottom = rect.top + self.px(19);
        self.text(
            item.hDC,
            action,
            rect,
            MUTED,
            Font::Small,
            DT_SINGLELINE | DT_WORD_ELLIPSIS,
        );
        if (item.itemState.0 & ODS_FOCUS.0) != 0 {
            let mut rect = item.rcItem;
            InflateRect(&mut rect, -2, -2);
            DrawFocusRect(item.hDC, &rect);
        }
    }
}

impl Drop for Theme {
    fn drop(&mut self) {
        unsafe {
            for font in &self.fonts {
                DeleteObject(font.get());
            }
            for brush in [self.background, self.surface, self.sidebar] {
                DeleteObject(brush);
            }
        }
    }
}
