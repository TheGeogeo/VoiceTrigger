# VoiceTrigger — AI contributor guidance

This file is versioned and must reflect the owner's project decisions. Local `tracking/`, personal settings, and AI working directories remain excluded from Git.

## Product goal

Build an economical Windows background application in Rust. A user-defined spoken phrase, defaulting to “Ok GPT”, runs a configured action. The primary action starts a new ChatGPT conversation, opens dictation, and submits after three seconds of silence when the app exposes the necessary controls.

## Required rules

- Keep the MIT license and repository `https://github.com/TheGeogeo/VoiceTrigger`.
- Read the owner's applicable instructions and mandatory installed skills before making changes.
- If local tracking exists, resume from `tracking/voice-trigger/TRACKING.md`. On a fresh clone, inspect the documentation and source, then create the local tracking required for the work. Do not treat compilation as proof of spoken recognition or successful ChatGPT submission.
- Write all source code, identifiers, comments, documentation, error messages, and application interface text in English. User-defined speech phrases and localized accessibility selectors are language data and retain their configured language.
- The owner authorizes feature branches, incremental commits and pushes, and merging verified milestones into `main` for this project. Follow the Git workflow below without requesting the same authorization again at each milestone.
- Keep recordings, transcripts, secrets, tokens, and personal machine paths out of versioned files. The requested live debug journal is bounded, memory-only, and never transmitted.
- Integration tests must not inspect or overwrite unrelated existing user drafts or use private conversation content as fixtures. Prefer control-only checks or explicitly authorized isolated synthetic data.
- Keep the native Triggers, Settings, Diagnostics and Credits navigation, shared Start/Stop controls, factual credits, keyboard access and DPI-aware layout. Preserve form values across page changes. Verify visual changes against actual rendered windows using an isolated configuration.
- Center button and sidebar labels, keep consistent text margins, and review minimum-size/scaled layouts for overlap and clipped labels. Long trigger rows use word-aware ellipsis; keep the complete value available in the editor. Do not add decorative or fabricated product copy.
- Native callbacks can synchronously re-enter the window procedure. Keep painting resources independent of mutable application state and guard command/event mutation with RefCell; release application borrows before modal native loops.
- Frequent live debug events are enabled only while Diagnostics > Activity is visible and Live debug is checked. Core action/error events remain available on other pages.
- Keep wake-trigger recognition local through SAPI. Do not add continuous rendering or a second permanent waveform capture at idle.
- Keep UI Automation scoped to ChatGPT. Do not introduce coordinate-based app integration or global keyboard shortcuts.
- Treat SAPI confidence `1` as high confidence, not an error. Confirm real recording controls before announcing readiness. The French `Arrêter la dictée` control confirms recording for transcription; `Annuler la dictée` cancels it.
- Treat SAPI phrase and text pointers as nullable even after a successful HRESULT. Check before reading or converting them, release owned COM allocations, and never dispatch a trigger for an empty result. Keep regression checks for this native crash boundary.
- The factory phrase is “Ok GPT”. Changing the factory default must preserve existing saved custom triggers.
- Update the relevant documentation in `doc/` as implementation advances.
- Check `.gitignore` and the actual staged files before publication. `ForAi.md` is intentionally versioned; other excluded local AI files must remain excluded.
- End Computer Use capture sessions after UI verification so the capture indicator is released.

## Required Git workflow

1. Fetch the latest remote references and start each feature, fix, or documentation change on a dedicated branch from an up-to-date `main`, such as `feature/live-debug`, `fix/speech-input`, or `docs/usage`.
2. Make focused commits at coherent, verified milestones. Use descriptive messages with prefixes such as `feat:`, `fix:`, `docs:`, and `test:`. Do not wait until the entire project is complete to preserve meaningful progress.
3. Update `README.md`, this file when project rules change, and the relevant `doc/` pages alongside changes to behavior, settings, commands, limitations, or validation.
4. Review the diff and staged file list. Version this guidance without sensitive data. Keep local tracking, AI working directories, personal settings, and recordings out of the index. Never use `git add -f` to bypass those exclusions.
5. Run checks appropriate to the change before merging. Distinguish compilation, synthetic recognition, real audio reception, actual spoken accuracy, and end-to-end ChatGPT interaction. Fix known regressions that prevent the feature from working before merging it.
6. Push the working branch at useful milestones. Once an increment is verified, squash its branch changes into one commit on `main` using `git merge --squash` followed by a commit, or the repository's squash-merge option. Keep incremental commits on the feature branch; do not rewrite already-published main history. Do not wait for every future project idea to be complete.
7. Fetch `main` again before the merge, preserve other contributors' work, resolve conflicts carefully, and recheck the result when code changes. Respect branch protection and use a pull request when required.
8. Write a detailed squash commit message: explain the original problem, scope, resulting behavior, significant implementation decisions, affected integration boundaries, verification actually run, and remaining limitations. Include concrete technical details that help review; never include secrets, personal data, or private test content.
9. Push the resulting `main` and verify its local commit matches the remote commit. Report the branch, commits, checks, squash, and push actually completed.

Force pushes, rewriting published history, deleting branches, and changing repository protections require separate instructions. The public workflow is also documented in `doc/development.md`.

## Release publication

Publish releases only when explicitly requested by the owner. Use an unused version tag on verified main and follow `doc/releasing.md`. Prepare artifacts with `scripts/package-release.ps1`: derive compiler-path remapping from the environment, verify the x64 executable, and package only explicit public files with dependency notices and SHA-256 sums. Never upload settings, test fixtures, logs, AI state or local build paths. Verify downloaded draft assets before publishing; preserve existing tags and published assets.

## Verification

```powershell
cargo fmt --check
cargo test --locked
cargo clippy --all-targets --locked -- -D warnings
cargo build --release --locked
```

Run ignored hardware tests explicitly when the target environment supports them. Keep detailed machine-specific evidence in local tracking and publish only accurate, generic validation information.
