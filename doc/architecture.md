# Architecture and resource use

## Rust components

- `src/config.rs`: trigger definitions, validation, and atomic JSON persistence.
- `src/silence.rs`: the platform-independent silence state machine.
- `src/journal.rs`: bounded in-memory debug history, without disk writes.
- `src/platform/ui.rs`: native application state, commands, accelerators, window lifecycle, hover/select subclassing and notification icon.
- `src/platform/ui/layout.rs`: page controls and the pure layout plan that decides each page's visible controls, cards, frames and dividers in logical pixels.
- `src/platform/ui/style.rs`: Light/Dark color tokens, fonts, DWM title-bar colors, buffered GDI painting, owner-drawn buttons, selects, list rows, switch, meter and the generated application icon.
- `src/platform/ui/tests.rs`: opt-in isolated native form/lifecycle checks and window-only rendering.
- `src/platform/speech.rs`: local SAPI recognition restricted to configured phrases.
- `src/platform/audio.rs`: temporary WinMM capture and RMS calculation.
- `src/platform/chatgpt.rs`: scoped ChatGPT window discovery and accessible control invocation.
- `src/platform/engine.rs`: background listening and dictation sequencing.

## Background operation

The interface sleeps in the Windows message loop. No embedded browser, continuous renderer, GUI timer, or periodic network request is added. The SAPI worker waits on a native event in 100 ms intervals so it can respond to cancellation. WinMM capture and UI Automation searches run only during an action or diagnostic.

With **Diagnostics > Activity** visible, **Live debug** displays SAPI audio and speech events with at most five level updates per second. Hiding the window or leaving Activity removes frequent events while keeping trigger recognition active. No additional permanent audio stream is opened. See [live debugging](debug.md).

SAPI itself uses resources to recognize speech. There is no promise of zero usage or a universal footprint; load depends on the Windows engine, microphone, and hardware. Measure the release build on the target machine.

## Dictation lifecycle

1. SAPI accepts a trigger and releases its microphone.
2. ChatGPT's window and New chat control become ready; navigation settles, dictation starts, and recording-stop/cancel controls are verified.
3. WinMM measures 50 ms microphone buffers without writing audio to disk.
4. Speech followed by three seconds of silence confirms dictation and then invokes Send.
5. A two-second cooldown precedes SAPI reactivation.

COM interfaces stay on their owning thread. Stop requests use atomics. Status and debug events reach the UI through a Rust channel and Win32 messages. Integration errors stop listening. Audio buffers are reset and released when recording ends.

## Verification boundaries

Tests cover invalid settings, duplicate phrases, persistence replacement, silence boundaries, bounded debug history, real microphone buffers, actual SAPI input attachment, live audio events, and synthetic phrase dispatch. Compilation or emulation alone does not prove spoken accuracy or compatibility with a particular ChatGPT version. Complete an oral test and review transcription on the target environment.

## Interface lifecycle

Triggers, Settings, Diagnostics and Credits share a single worker and persistent form controls. Page changes show/hide existing controls rather than rebuilding state. The window has no menu bar; accelerators use the same guarded commands as buttons. Editing and saving are disabled during listening, including command paths reached through keyboard shortcuts.

One layout plan per page is the single source for control placement, visibility and painting, so cards, field frames and controls cannot drift apart. Unit tests check that no control leaves the window, crosses a card edge or overlaps another at the minimum, default and large sizes. The minimum window size is clamped to the monitor work area.

The Light and Dark palettes are color tokens used by every painted surface and `WM_CTLCOLOR*` response. Switching the theme swaps the palette and brushes, sets the DWM title-bar colors (`DWMWA_USE_IMMERSIVE_DARK_MODE`, caption, text and border colors; older Windows versions ignore unsupported attributes) and the scroll-bar visual style, then repaints. Only the appearance preference is written to the configuration at that moment.

The window uses per-monitor DPI awareness, scales Segoe UI fonts, icons and layout, and handles Windows' suggested DPI-change rectangle. GDI painting runs on demand through an off-screen buffer; there is no animation loop. Native buttons, list boxes, combo boxes and edit controls retain Windows keyboard semantics, with visible focus indicators. Edit controls are borderless inside painted frames; combo boxes keep their native behavior and are repainted by a subclass. A small paint-only view state (page, tab, listening, Live debug, meter level and hovered control) is shared with painting and subclass callbacks; it never borrows the RefCell-protected application state, so synchronous native painting cannot create an overlapping mutable application reference. Window destruction precedes resource cleanup.

MSVC builds embed the application manifest from `resources/voice-trigger.manifest` through `build.rs`, enabling Common Controls v6 and declaring per-monitor DPI awareness. See Microsoft’s [application manifest reference](https://learn.microsoft.com/en-us/windows/win32/sbscs/application-manifests). Native notifications deferred during a reentrant application borrow are reposted after the outer callback releases it, preserving final worker events without a polling timer.
