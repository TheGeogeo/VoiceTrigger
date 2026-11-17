# Desktop interface

VoiceTrigger uses a native, resizable Windows interface. The sidebar switches pages; the footer keeps listening status, **Save configuration**, **Hide to tray**, **Stop**, and **Start listening** available. The layout follows Windows display scaling.

Button and sidebar labels are centered with consistent internal margins. Long trigger rows use word-aware ellipsis; selecting the row exposes the complete phrase in the editor. Help text wraps, while longer reports and the complete license scroll inside their own panels.

## Triggers

The left list shows each configured phrase and action. Select a row to edit its details on the right. **Target** is enabled for website and executable actions.

- **Apply changes** updates the selected row in memory.
- **Add as new** creates a separate row from the current fields. Use a distinct phrase.
- **Remove** removes the selected row; at least one trigger must remain.
- **Save configuration** applies the selected row and writes all triggers and settings to the configuration file.

Page navigation retains the editor fields. Apply edits before selecting another trigger. Stop listening before editing or saving; disabled controls and command guards prevent changes to the active configuration.

## Settings

**Recognition** contains the installed-engine language and energy threshold used to detect silence. **ChatGPT** contains the optional executable path. Existing settings and custom trigger phrases are preserved by the redesign. The dictation sequence and three-second silence interval are unchanged.

## Diagnostics

**Activity** shows the microphone level, **Live debug**, and bounded event log. Frequent level/sound/hypothesis events run only while this view is visible and Live debug is checked. Core trigger, action and error events are still recorded in memory when another page is selected. **Clear log** empties that journal.

**System checks** has a **Run checks** button and a separate full report. It inspects installed SAPI engines and accessible ChatGPT controls without starting dictation. Reports remain readable without replacing the activity log or compressing the full report into the footer. See [debugging](debug.md) and [privacy](privacy.md).

## Credits

Credits shows the project version, TheGeogeo attribution, repository/documentation links, open-source libraries and the complete project MIT license. Version text comes from Cargo package metadata; copyright comes from the repository license. Current library metadata:

| Library | Version | License |
| --- | --- | --- |
| windows-rs (`windows`) | 0.48.0 | MIT OR Apache-2.0 |
| Serde | 1.0.188 | MIT OR Apache-2.0 |
| serde_json | 1.0.105 | MIT OR Apache-2.0 |

## Menus and keyboard

**File** contains configuration saving, listening controls, hiding and quitting. **View** switches pages. **Help** opens documentation, source code or Credits.

| Shortcut | Action |
| --- | --- |
| Ctrl+1 | Triggers |
| Ctrl+2 | Settings |
| Ctrl+3 | Diagnostics |
| Ctrl+4 | Credits |
| Ctrl+S | Save configuration while stopped |
| F1 | Credits |
| Tab / Shift+Tab | Move between visible controls |
| Space | Activate a focused button or checkbox |

Closing or minimizing still hides the window in the notification area. Double-click its icon to restore it; use **File > Quit** or the icon's **Quit** command to exit. Navigation never starts another listener or sends a ChatGPT message.

## Native interface verification

The opt-in desktop test uses a separate configuration and captures only its own native test window. It checks page retention, add/save/remove, the shared listening controls, hide/restore and an untruncated diagnostic report. It also renders all pages, the minimum layout and simulated 125%/150% DPI-change messages for visual review:

```powershell
cargo test --release --locked native_pages_preserve_configuration_and_render -- --ignored --test-threads=1 --nocapture
```

It requires an interactive Windows desktop, the French SAPI engine and a microphone. Captures and synthetic settings go under ignored `target/ui-verification/`. It does not use personal settings or send a message. Simulated DPI messages exercise the application's scaling path; they do not replace a real multi-monitor display-scaling test.
