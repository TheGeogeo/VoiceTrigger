# VoiceTrigger

A native Windows application written in Rust that runs actions when you say a custom trigger phrase. The default phrase is **“Ok GPT”**.

VoiceTrigger uses the local Windows SAPI recognizer and stays in the notification area. No downloaded recognition model or API key is required. Source code, comments, application messages, and documentation are in English. Configured speech phrases and localized ChatGPT accessibility labels retain their original language.

## Getting started

Requirements: Windows 10/11, a working default microphone, and an installed SAPI recognition engine for the selected language. Building requires Rust 1.72 or newer and the MSVC build tools.

```powershell
cargo build --release --locked
.\target\release\voice-trigger.exe
```

1. Edit the trigger phrase and select its action.
2. Click **Save**, then **Start listening**.
3. Check the microphone level and **Live debug** journal. Say **“Ok GPT”** and look for **Recognized**, **Trigger detected**, and **Running action**.
4. For ChatGPT dictation, wait for the readiness tone before asking your question.
5. After speech, three seconds of silence confirm dictation and send the transcription when ChatGPT exposes the required controls.
6. Click **Hide to tray** to keep listening in the background. Closing or minimizing the window also hides it.

Double-click the notification icon to restore the window. Its menu can stop listening or quit. **Quit** closes the application completely. Only one instance runs at a time; launching it again restores the existing window. Listening starts only after **Start listening** is clicked.

**“Ok GPT”** is the default for new configurations. Existing saved trigger phrases are preserved. Edit and save your selected trigger if you want to change an existing configuration.

## Available actions

- Start dictation in a new ChatGPT conversation.
- Open ChatGPT.
- Open an HTTP(S) address in the default browser.
- Launch an existing `.exe` file, without arguments or a command interpreter.

ChatGPT integration depends on the installed app's accessible controls. Open ChatGPT, sign in, grant its microphone access yourself, and select the **Chat** view. **Diagnostics** checks recognition engines, window discovery, and known accessible controls without starting dictation. Native dictation startup, cancellation, and simultaneous silence monitoring have been verified on the installed app. Full transcription and message submission remain separate target-environment checks.

## Documentation

- [Usage and troubleshooting](doc/usage.md)
- [Live debugging and microphone detection](doc/debug.md)
- [ChatGPT integration and limitations](doc/chatgpt.md)
- [Architecture and resource use](doc/architecture.md)
- [Privacy and Git exclusions](doc/privacy.md)
- [Development, branches, and delivery](doc/development.md)
- [Project guidance for AI contributors](ForAi.md)

## Development checks

```powershell
cargo fmt --check
cargo test --locked
cargo clippy --all-targets --locked -- -D warnings
cargo build --release --locked
cargo run -- --diagnostic --config .\target\diagnostic-settings.json
```

Hardware and desktop integration tests are ignored by default. The full suite requires Windows, the French SAPI engine, an accessible microphone, and the signed-in ChatGPT desktop app. Its native tests restore ChatGPT, open a new conversation, briefly start dictation, and cancel it without sending:

```powershell
cargo test --locked -- --include-ignored --test-threads=1
```

These checks include real input attachment, live audio events, synthetic phrase dispatch, accessible recording controls, and a silence monitor running alongside ChatGPT. Text emulation does not establish real spoken accuracy. Tests neither inspect existing user drafts nor send a message to ChatGPT.

`Cargo.lock` is versioned for reproducible application builds. The command-line diagnostic writes a `.diagnostic.txt` file next to the selected configuration. The release executable is a GUI application, so use this file to read the report.

License: [MIT](LICENSE).
