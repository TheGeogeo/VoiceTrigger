# Architecture and resource use

## Rust components

- `src/config.rs`: trigger definitions, validation, and atomic JSON persistence.
- `src/silence.rs`: the platform-independent silence state machine.
- `src/journal.rs`: bounded in-memory debug history, without disk writes.
- `src/platform/ui.rs`: native Win32 controls and notification icon.
- `src/platform/speech.rs`: local SAPI recognition restricted to configured phrases.
- `src/platform/audio.rs`: temporary WinMM capture and RMS calculation.
- `src/platform/chatgpt.rs`: scoped ChatGPT window discovery and accessible control invocation.
- `src/platform/engine.rs`: background listening and dictation sequencing.

## Background operation

The interface sleeps in the Windows message loop. No embedded browser, continuous renderer, GUI timer, or periodic network request is added. The SAPI worker waits on a native event in 100 ms intervals so it can respond to cancellation. WinMM capture and UI Automation searches run only during an action or diagnostic.

With the window visible, **Live debug** displays SAPI audio and speech events with at most five level updates per second. Hiding the window removes frequent events while keeping trigger recognition active. No additional permanent audio stream is opened. See [live debugging](debug.md).

SAPI itself uses resources to recognize speech. There is no promise of zero usage or a universal footprint; load depends on the Windows engine, microphone, and hardware. Measure the release build on the target machine.

## Dictation lifecycle

1. SAPI accepts a trigger and releases its microphone.
2. ChatGPT opens a new conversation and starts dictation.
3. WinMM measures 50 ms microphone buffers without writing audio to disk.
4. Speech followed by three seconds of silence confirms dictation and then invokes Send.
5. A two-second cooldown precedes SAPI reactivation.

COM interfaces stay on their owning thread. Stop requests use atomics. Status and debug events reach the UI through a Rust channel and Win32 messages. Integration errors stop listening. Audio buffers are reset and released when recording ends.

## Verification boundaries

Tests cover invalid settings, duplicate phrases, persistence replacement, silence boundaries, bounded debug history, real microphone buffers, actual SAPI input attachment, live audio events, and synthetic phrase dispatch. Compilation or emulation alone does not prove spoken accuracy or compatibility with a particular ChatGPT version. Complete an oral test and review transcription on the target environment.
