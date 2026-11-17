# Windows releases

Publishing a GitHub release requires an explicit owner request. Feature-branch merge authorization alone does not publish a release.

## Prepare the source

Complete the normal feature-branch checks and detailed squash workflow. Confirm that local and remote main identify the intended release source, and choose an unused tag matching the Cargo package version. Never move an existing release tag or replace published assets silently.

## Build portable assets

Run the packaging script from the repository with PowerShell 7, the installed Rust/MSVC toolchain and cached locked dependencies:

```powershell
pwsh -File scripts/package-release.ps1
```

The script creates a new ignored `dist/v<version>/` directory and refuses to overwrite it. Use `-OutputDirectory dist/<new-name>` for a new attempt. Build-path remapping is derived from the current workspace/user profile and applied only to child Cargo processes. No personal path is written into the repository.

It runs ordinary tests and the opt-in native UI test with the same remapping flags, then builds Windows x64 in a separate target directory. An interactive desktop, French SAPI engine and microphone are required by the UI test. It does not send a ChatGPT message.

The script checks the PE machine type and absence of the current machine's user/workspace build paths. Precompiled Rust libraries can retain upstream distribution build paths; any remaining user-shaped root must also be found independently in those installed libraries. Unknown roots fail the check. It explicitly packages the executable, portable instructions, public documentation, project MIT license and original dependency/Rust license notices. Personal settings, AI instructions, logs, debug symbols and working directories are excluded. Review the archive inventory and `build-receipt.json` before upload.

Upload only these release assets:

- `voice-trigger.exe`
- `VoiceTrigger-v<version>-windows-x86_64.zip`
- `SHA256SUMS.txt`

The ZIP is recommended because it includes documentation and license notices. The standalone executable is convenient for replacing an existing installation; retain its existing documentation and notices. Preserve custom settings under `%APPDATA%\VoiceTrigger`.

The MSVC executable imports `VCRUNTIME140.dll`; document the x64 Microsoft Visual C++ v14 runtime as a prerequisite and use Microsoft's [official runtime downloads](https://learn.microsoft.com/en-us/cpp/windows/latest-supported-vc-redist). Do not bundle DLLs copied from a personal Windows installation.

## Publish and verify

Prepare English release notes describing actual features, requirements and verification limits. Create an annotated version tag on verified main and push that exact tag. Create a draft release with `gh release create --draft --verify-tag --notes-file <file>` and the three explicit assets.

Read back the release metadata, download its assets into a new ignored directory, and compare their SHA-256 hashes with the local originals. Publish the verified draft using `gh release edit <tag> --draft=false --latest`, then verify the public release URL, non-draft status, tag target and asset inventory.

Keep authentication tokens, local metadata paths and private diagnostic data out of release notes and attachments. Do not claim hardware or end-to-end checks that were not run. GitHub's source archives do not replace the uploaded Windows executable.
