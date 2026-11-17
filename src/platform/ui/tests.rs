use super::*;
use std::time::Duration;
use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_EXTENDED_FRAME_BOUNDS};
use windows::Win32::Storage::Xps::{PrintWindow, PRINT_WINDOW_FLAGS};

unsafe fn pump() {
    let mut message = MSG::default();
    while PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
        if message.message != WM_QUIT {
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
}

unsafe fn command(desktop: &Desktop, id: u16) {
    SendMessageW(desktop.window(), WM_COMMAND, WPARAM(id as usize), LPARAM(0));
    pump();
}

unsafe fn resize(desktop: &Desktop, width: i32, height: i32) {
    let theme = &desktop.state.theme;
    let mut rect = RECT {
        right: theme.px(width),
        bottom: theme.px(height),
        ..Default::default()
    };
    AdjustWindowRectExForDpi(
        &mut rect,
        WS_OVERLAPPEDWINDOW,
        false,
        WS_EX_CONTROLPARENT,
        theme.dpi.get(),
    );
    SetWindowPos(
        desktop.window(),
        None,
        40,
        40,
        rect.right - rect.left,
        rect.bottom - rect.top,
        SWP_NOZORDER | SWP_NOACTIVATE,
    );
    pump();
}

/// Saves the visible window frame only. The invisible resize borders that
/// PrintWindow also renders are cropped using DWM's extended frame bounds.
unsafe fn snapshot(desktop: &Desktop, name: &str) {
    let window = desktop.window();
    RedrawWindow(
        window,
        None,
        None,
        RDW_INVALIDATE | RDW_ALLCHILDREN | RDW_UPDATENOW | RDW_ERASE | RDW_FRAME,
    );
    std::thread::sleep(Duration::from_millis(120));
    pump();
    let mut rect = RECT::default();
    GetWindowRect(window, &mut rect);
    let mut frame = rect;
    let _ = DwmGetWindowAttribute(
        window,
        DWMWA_EXTENDED_FRAME_BOUNDS,
        (&mut frame as *mut RECT).cast(),
        std::mem::size_of::<RECT>() as u32,
    );
    let width = rect.right - rect.left;
    let height = rect.bottom - rect.top;
    let screen = GetWindowDC(window);
    let dc = CreateCompatibleDC(screen);
    let bitmap = CreateCompatibleBitmap(screen, width, height);
    let old = SelectObject(dc, bitmap);
    let rendered = PrintWindow(window, dc, PRINT_WINDOW_FLAGS(PW_RENDERFULLCONTENT));
    SelectObject(dc, old);
    let mut info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width,
            biHeight: -height,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0 as u32,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut pixels = vec![0u8; (width * height * 4) as usize];
    let rows = GetDIBits(
        dc,
        bitmap,
        0,
        height as u32,
        Some(pixels.as_mut_ptr().cast()),
        &mut info,
        DIB_RGB_COLORS,
    );
    DeleteObject(bitmap);
    DeleteDC(dc);
    ReleaseDC(window, screen);
    assert!(rendered.as_bool(), "The native test window must render");
    assert_eq!(rows, height, "The complete window must be captured");
    let left = (frame.left - rect.left).clamp(0, width - 1);
    let top = (frame.top - rect.top).clamp(0, height - 1);
    let right = (frame.right - rect.left).clamp(left + 1, width);
    let bottom = (frame.bottom - rect.top).clamp(top + 1, height);
    let (crop_width, crop_height) = (right - left, bottom - top);
    let mut cropped = Vec::with_capacity((crop_width * crop_height * 4) as usize);
    // BMP rows are stored bottom-up.
    for row in (top..bottom).rev() {
        let start = ((row * width + left) * 4) as usize;
        cropped.extend_from_slice(&pixels[start..start + crop_width as usize * 4]);
    }
    let mut file = Vec::new();
    file.extend_from_slice(b"BM");
    file.extend_from_slice(&(54u32 + cropped.len() as u32).to_le_bytes());
    file.extend_from_slice(&[0u8; 4]);
    file.extend_from_slice(&54u32.to_le_bytes());
    file.extend_from_slice(&40u32.to_le_bytes());
    file.extend_from_slice(&crop_width.to_le_bytes());
    file.extend_from_slice(&crop_height.to_le_bytes());
    file.extend_from_slice(&1u16.to_le_bytes());
    file.extend_from_slice(&32u16.to_le_bytes());
    file.extend_from_slice(&[0u8; 24]);
    file.extend_from_slice(&cropped);
    std::fs::write(format!("target/ui-verification/{name}.bmp"), file).unwrap();
}

unsafe fn visible(desktop: &Desktop, id: u16) -> bool {
    IsWindowVisible(desktop.state.app.borrow().child(id)).as_bool()
}

#[test]
#[ignore = "Creates an isolated native window, tests form commands, briefly listens, and saves window-only captures under target"]
fn native_pages_preserve_configuration_and_render() {
    unsafe {
        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        std::fs::create_dir_all("target/ui-verification").unwrap();
        let path = PathBuf::from("target/ui-verification/settings.json");
        let _ = std::fs::remove_file(&path);
        let desktop = Desktop::new(
            Settings::default(),
            path.clone(),
            w!("VoiceTriggerVerificationWindow"),
            "VoiceTrigger",
        )
        .unwrap();
        ShowWindow(desktop.window(), SW_SHOWNOACTIVATE);
        pump();
        assert!(
            GetMenu(desktop.window()).0 == 0,
            "The unified window has no native menu bar"
        );
        let dc = GetDC(desktop.window());
        for font in [Font::Body, Font::Label, Font::Brand] {
            let old = SelectObject(dc, desktop.state.theme.font(font));
            let mut name = [0u16; 128];
            let count = GetTextFaceW(dc, Some(&mut name));
            println!(
                "Native UI typeface: {}",
                String::from_utf16_lossy(&name[..count.saturating_sub(1) as usize])
            );
            SelectObject(dc, old);
        }
        ReleaseDC(desktop.window(), dc);
        let shortcuts = accelerators().unwrap();
        let key = MSG {
            hwnd: desktop.window(),
            message: WM_KEYDOWN,
            wParam: WPARAM(0x70),
            ..Default::default()
        };
        assert_ne!(
            TranslateAcceleratorW(desktop.window(), shortcuts.0, &key),
            0
        );
        assert!(
            desktop.state.view.page.get() == Page::Credits,
            "F1 must navigate through native accelerator dispatch"
        );
        assert!(visible(&desktop, LICENSE_TEXT) && !visible(&desktop, LIST));
        command(&desktop, NAV_TRIGGERS);
        {
            let app = desktop.state.app.borrow();
            app.set_text(PHRASE, "Interface verification");
            SendMessageW(app.child(ACTION), CB_SETCURSEL, WPARAM(2), LPARAM(0));
            app.set_text(TARGET, "https://example.com");
        }
        command(&desktop, ADD);
        assert_eq!(desktop.state.app.borrow().settings.triggers.len(), 2);
        command(&desktop, NAV_SETTINGS);
        desktop.state.app.borrow().set_text(THRESHOLD, "0.02");
        command(&desktop, NAV_CREDITS);
        command(&desktop, NAV_TRIGGERS);
        assert_eq!(
            desktop.state.app.borrow().text(PHRASE),
            "Interface verification"
        );
        command(&desktop, SAVE);
        let saved = Settings::load(&path).unwrap();
        assert_eq!(saved.triggers.len(), 2);
        assert_eq!(saved.voice_threshold, 0.02);
        assert_eq!(saved.triggers[1].action.target(), "https://example.com");
        assert_eq!(saved.appearance, Appearance::Light);
        command(&desktop, DELETE);
        assert_eq!(desktop.state.app.borrow().settings.triggers.len(), 1);

        // The theme applies at once and persists without saving pending edits.
        command(&desktop, NAV_SETTINGS);
        command(&desktop, THEME_DARK);
        assert!(desktop.state.theme.palette().dark);
        let saved = Settings::load(&path).unwrap();
        assert_eq!(saved.appearance, Appearance::Dark);
        assert_eq!(
            saved.triggers.len(),
            2,
            "Unsaved trigger edits stay out of the file"
        );
        command(&desktop, THEME_LIGHT);
        assert!(!desktop.state.theme.palette().dark);
        assert_eq!(Settings::load(&path).unwrap().appearance, Appearance::Light);
        command(&desktop, NAV_TRIGGERS);

        // A long, dedicated test phrase prevents ordinary wake phrases from
        // starting a ChatGPT action during this brief listener lifecycle check.
        desktop
            .state
            .app
            .borrow()
            .set_text(PHRASE, "Interface verification alpha omega complete");
        command(&desktop, START);
        let deadline = Instant::now() + Duration::from_secs(10);
        while !desktop
            .state
            .app
            .borrow()
            .text(STATUS)
            .starts_with("Listening")
            && Instant::now() < deadline
        {
            std::thread::sleep(Duration::from_millis(25));
            pump();
        }
        assert!(
            desktop.state.app.borrow().worker.is_some(),
            "Listening must remain active"
        );
        assert!(visible(&desktop, STOP) && !visible(&desktop, START));
        command(&desktop, NAV_SETTINGS);
        assert!(!IsWindowEnabled(desktop.state.app.borrow().child(THRESHOLD)).as_bool());
        assert!(IsWindowEnabled(desktop.state.app.borrow().child(STOP)).as_bool());
        assert!(IsWindowEnabled(desktop.state.app.borrow().child(THEME_DARK)).as_bool());
        command(&desktop, NAV_DIAGNOSTICS);
        assert!(desktop.state.app.borrow().debug_enabled());
        command(&desktop, DEBUG);
        assert!(!desktop.state.app.borrow().debug_enabled());
        command(&desktop, DEBUG);
        assert!(desktop.state.app.borrow().debug_enabled());
        command(&desktop, HIDE);
        assert!(!IsWindowVisible(desktop.window()).as_bool());
        assert!(!desktop.state.app.borrow().debug_enabled());
        command(&desktop, OPEN);
        assert!(IsWindowVisible(desktop.window()).as_bool());
        assert!(desktop.state.app.borrow().worker.is_some());
        command(&desktop, STOP);
        let deadline = Instant::now() + Duration::from_secs(8);
        while desktop.state.app.borrow().worker.is_some() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(25));
            pump();
        }
        assert!(desktop.state.app.borrow().worker.is_none());
        assert!(visible(&desktop, START) && !visible(&desktop, STOP));
        command(&desktop, NAV_TRIGGERS);
        desktop.state.app.borrow().set_text(PHRASE, "Ok GPT");
        desktop.state.app.borrow().set_text(THRESHOLD, "0.015");
        command(&desktop, SAVE);
        desktop.state.app.borrow_mut().journal.clear();
        desktop.state.app.borrow().set_text(LOG, "");

        let report = "Verification report\r\n".repeat(100);
        for (theme, id) in [("light", THEME_LIGHT), ("dark", THEME_DARK)] {
            command(&desktop, NAV_SETTINGS);
            command(&desktop, id);
            resize(&desktop, layout::DEFAULT_WIDTH, layout::DEFAULT_HEIGHT);
            desktop.state.app.borrow().set_text(
                STATUS,
                "Ready. Start listening to enable your voice triggers.",
            );
            for (id, name) in [
                (NAV_TRIGGERS, "triggers"),
                (NAV_SETTINGS, "settings"),
                (NAV_DIAGNOSTICS, "activity"),
                (NAV_CREDITS, "credits"),
            ] {
                command(&desktop, id);
                if id == NAV_DIAGNOSTICS {
                    command(&desktop, ACTIVITY_TAB);
                    assert_eq!(
                        desktop.state.app.borrow().text(LEVEL),
                        "Microphone: stopped"
                    );
                }
                snapshot(&desktop, &format!("{theme}-{name}"));
            }
            command(&desktop, NAV_DIAGNOSTICS);
            command(&desktop, CHECKS_TAB);
            let window = desktop.window();
            {
                let app = desktop.state.app.borrow_mut();
                app.events
                    .sender
                    .send(Event::DiagnosticReport(report.clone()))
                    .unwrap();
                SendMessageW(window, EVENT_MESSAGE, WPARAM(0), LPARAM(0));
            }
            SendMessageW(desktop.window(), WM_NULL, WPARAM(0), LPARAM(0));
            pump();
            assert_eq!(desktop.state.app.borrow().text(REPORT), report);
            snapshot(&desktop, &format!("{theme}-checks"));
            command(&desktop, ACTIVITY_TAB);
            resize(&desktop, layout::MIN_WIDTH, layout::MIN_HEIGHT);
            for (id, name) in [
                (NAV_TRIGGERS, "triggers"),
                (NAV_SETTINGS, "settings"),
                (NAV_DIAGNOSTICS, "activity"),
                (NAV_CREDITS, "credits"),
            ] {
                command(&desktop, id);
                snapshot(&desktop, &format!("{theme}-{name}-minimum"));
            }
            resize(&desktop, 1600, 960);
            command(&desktop, NAV_TRIGGERS);
            snapshot(&desktop, &format!("{theme}-triggers-large"));
        }
        command(&desktop, NAV_SETTINGS);
        command(&desktop, THEME_LIGHT);
        for dpi in [120u32, 144] {
            let mut rect = RECT {
                left: 40,
                top: 40,
                right: 40 + 1100 * dpi as i32 / 96,
                bottom: 40 + 760 * dpi as i32 / 96,
            };
            SendMessageW(
                desktop.window(),
                WM_DPICHANGED,
                WPARAM((dpi | (dpi << 16)) as usize),
                LPARAM(&mut rect as *mut _ as isize),
            );
            resize(&desktop, layout::DEFAULT_WIDTH, layout::DEFAULT_HEIGHT);
            for (id, name) in [(NAV_TRIGGERS, "triggers"), (NAV_SETTINGS, "settings")] {
                command(&desktop, id);
                snapshot(&desktop, &format!("light-{name}-dpi-{dpi}"));
            }
        }
        println!("Native UI: navigation, isolated add/save/remove, immediate theme persistence, retained values, Start/Stop, live-debug toggle, hide/restore, full diagnostic report and window rendering passed.");
    }
}
