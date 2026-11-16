# Privacy and Git exclusions

Wake-trigger recognition uses the SAPI engine installed in Windows. VoiceTrigger requires no API key, integrates no cloud wake-word service, and sends no recording to its own server. Temporary silence-detection buffers stay in memory and are not saved.

Once ChatGPT dictation begins, **ChatGPT** processes audio and the question under its own services, settings, and terms. Local wake-trigger recognition does not make ChatGPT transcription local.

Settings contain custom phrases, URLs, and possibly local executable paths. By default they are stored outside the repository in `%APPDATA%\VoiceTrigger`. Diagnostic report files contain no conversation transcript, but can contain a local configuration path. Review reports before sharing them.

**Live debug** can show recognition hypotheses and matches in a bounded memory-only journal. It does not retain ChatGPT prompt audio, write a log file, or transmit journal contents. Clearing the journal or quitting discards it.

The `.gitignore` excludes:

- Build and distribution outputs.
- Personal settings, diagnostics, logs, and temporary files.
- `.env` files, private keys, and certificates.
- Claude, Codex, and agent working directories.
- Other local AI instructions such as `AGENTS.md`, `CLAUDE.md`, `GEMINI.md`, `AI.md`, and `*.ai.md`.
- Local `tracking/` and editor configuration.

`ForAi.md` is intentionally versioned at the owner's request. Keep its guidance generic and free of sensitive data. `Cargo.lock`, source, `README.md`, `doc/`, and the MIT license are also versioned. Ignore patterns do not remove secrets already tracked: inspect `git diff --cached` before publication. Documentation examples must not include credentials, personal paths, or conversation data.
