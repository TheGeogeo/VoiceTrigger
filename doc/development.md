# Development and delivery

## Language and branches

Write all source code, identifiers, comments, error messages, interface text, and documentation in English. Configured spoken phrases and localized accessibility labels are language data and retain the required language.

Use a dedicated branch for each feature, fix, or documentation change, starting from an up-to-date `main`. Examples include `feature/live-debug`, `fix/speech-input`, and `docs/usage`.

Make focused commits at coherent milestones using descriptive prefixes such as `feat:`, `fix:`, `docs:`, and `test:`. Push useful progress on the working branch. In-progress work belongs on that branch; `main` contains verified functional increments.

## Documentation with implementation

Update `README.md`, relevant `doc/` pages, and `ForAi.md` when applicable project rules change. Document behavior, configuration, commands, architecture, validation, and limitations alongside the implementation rather than postponing them until project completion.

Separate automated checks, hardware observations, synthetic recognition, actual spoken accuracy, and end-to-end ChatGPT interaction. Do not present untested integration as verified.

## Checks before merging

For Rust changes:

```powershell
cargo fmt --check
cargo test --locked
cargo clippy --all-targets --locked -- -D warnings
cargo build --release --locked
```

Add a focused check for the changed behavior. Ignored hardware/integration tests must be explicitly run on a machine with the required microphone and engine. Serialize device tests when running the whole suite:

```powershell
cargo test --locked -- --include-ignored --test-threads=1
```

The full suite includes a 220-second real-microphone SAPI soak, with a recognizer restart and both live-debug settings. See [native crash diagnostics](debug.md#unexpected-application-termination) for the original null-pointer regression and a focused soak command. When only SAPI changes, unrelated ChatGPT desktop tests can be skipped explicitly; report those omissions rather than claiming a complete desktop rerun.

Documentation-only changes require review of content, commands, and links; they do not need tests that mirror the text.

Before committing or pushing, inspect staged files and [privacy exclusions](privacy.md). `ForAi.md` is versioned without sensitive information. Local tracking, AI working directories, and personal configurations remain excluded. Preserve a configured GitHub no-reply commit address when email privacy is required.

## Merge and publication

Fetch the latest `main`, integrate changes into the feature branch if needed, and verify the result. Keep incremental commits on the working branch. Once the increment is ready, use `git merge --squash` and commit its combined changes, or use GitHub's squash-merge option. Respect branch protection and use a pull request when required. Do not rewrite previously published main history to retrofit this policy.

The squash commit message must be detailed: describe the original failure or need, the implemented behavior, meaningful design choices, affected components, checks actually executed, and remaining limitations. Explain concrete triggers and outcomes when they help a reviewer. Keep private data, secrets, and unrelated work out of the message.

Push `main` after merging and compare local and remote commit identifiers. Delivery reports must identify actual commits, checks, and remaining limits. Force pushes, rewriting published history, deleting branches, or changing protection rules are not part of the normal workflow and require separate instructions.
