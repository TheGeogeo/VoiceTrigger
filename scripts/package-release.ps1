#requires -Version 7.0
[CmdletBinding()]
param([string]$OutputDirectory = '')

$ErrorActionPreference = 'Stop'
$workspace = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
Push-Location $workspace
$oldEncodedFlags = $env:CARGO_ENCODED_RUSTFLAGS
try {
    $metadataText = & cargo metadata --offline --locked --format-version 1 --filter-platform x86_64-pc-windows-msvc
    if ($LASTEXITCODE -ne 0) { throw 'Could not read locked Cargo metadata.' }
    $metadata = $metadataText | ConvertFrom-Json
    $project = $metadata.packages | Where-Object { $_.name -eq 'voice-trigger' }
    $version = $project.version
    if ($version -notmatch '^\d+\.\d+\.\d+$') { throw 'Expected a stable numeric package version.' }
    if (-not $OutputDirectory) { $OutputDirectory = "dist/v$version" }
    $output = [IO.Path]::GetFullPath((Join-Path $workspace $OutputDirectory))
    $dist = [IO.Path]::GetFullPath((Join-Path $workspace 'dist')) + [IO.Path]::DirectorySeparatorChar
    if (-not $output.StartsWith($dist, [StringComparison]::OrdinalIgnoreCase)) {
        throw 'Release output must be inside the repository dist directory.'
    }
    if (Test-Path -LiteralPath $output) { throw 'Release output already exists. Use a new output directory.' }
    [IO.Directory]::CreateDirectory($output) | Out-Null

    # Keep machine-specific prefixes in this process only, never in tracked files.
    $flags = @()
    if ($oldEncodedFlags) { $flags += $oldEncodedFlags.Split([char]31) }
    elseif ($env:RUSTFLAGS) { $flags += $env:RUSTFLAGS.Split(' ', [StringSplitOptions]::RemoveEmptyEntries) }
    foreach ($prefix in @($env:USERPROFILE, $workspace)) {
        if (-not $prefix) { throw 'A build path prefix could not be determined.' }
        $replacement = if ($prefix -eq $workspace) { 'voice-trigger' } else { 'build-user' }
        $flags += "--remap-path-prefix=$prefix=$replacement"
        $flags += "--remap-path-prefix=$($prefix.Replace('\', '/'))=$replacement"
    }
    $env:CARGO_ENCODED_RUSTFLAGS = $flags -join [char]31
    $target = 'target/publish'
    $commands = @(
        @('test', '--release', '--offline', '--locked', '--target', 'x86_64-pc-windows-msvc', '--target-dir', $target, '--', '--test-threads=1'),
        @('test', '--release', '--offline', '--locked', '--target', 'x86_64-pc-windows-msvc', '--target-dir', $target, 'native_pages_preserve_configuration_and_render', '--', '--ignored', '--test-threads=1', '--nocapture'),
        @('build', '--release', '--offline', '--locked', '--target', 'x86_64-pc-windows-msvc', '--target-dir', $target)
    )
    for ($index = 0; $index -lt $commands.Count; $index++) {
        & cargo @($commands[$index]) 2>&1 | Tee-Object -FilePath (Join-Path $output "check-$index.txt")
        if ($LASTEXITCODE -ne 0) { throw "Release check $index failed." }
    }

    $binary = Join-Path $workspace "$target/x86_64-pc-windows-msvc/release/voice-trigger.exe"
    $bytes = [IO.File]::ReadAllBytes($binary)
    $peOffset = [BitConverter]::ToInt32($bytes, 0x3c)
    if ([BitConverter]::ToUInt16($bytes, $peOffset + 4) -ne 0x8664) { throw 'Expected a Windows x86_64 executable.' }
    $binaryTexts = @([Text.Encoding]::UTF8.GetString($bytes), [Text.Encoding]::Unicode.GetString($bytes), [Text.Encoding]::Unicode.GetString($bytes, 1, $bytes.Length - 1))
    $remainingRoots = @()
    foreach ($text in $binaryTexts) {
        if ($text.IndexOf($env:USERPROFILE, [StringComparison]::OrdinalIgnoreCase) -ge 0 -or
            $text.IndexOf($env:USERPROFILE.Replace('\', '/'), [StringComparison]::OrdinalIgnoreCase) -ge 0 -or
            $text.IndexOf($workspace, [StringComparison]::OrdinalIgnoreCase) -ge 0 -or
            $text.IndexOf($workspace.Replace('\', '/'), [StringComparison]::OrdinalIgnoreCase) -ge 0) {
            throw 'The release binary still contains a machine-specific build path.'
        }
        $remainingRoots += [regex]::Matches($text, '(?i)[A-Z]:[\\/]Users[\\/][^\\/\x00]+') | ForEach-Object { $_.Value }
    }
    # Precompiled Rust libraries can retain their upstream distribution build
    # paths. Accept those only when independently present in the installed rlibs.
    $remainingRoots = @($remainingRoots | Sort-Object -Unique)
    $sysroot = (& rustc --print sysroot).Trim()
    if ($LASTEXITCODE -ne 0) { throw 'Rust toolchain location is unavailable.' }
    $archives = @(Get-ChildItem -LiteralPath (Join-Path $sysroot 'lib/rustlib/x86_64-pc-windows-msvc/lib') -Filter '*.rlib' -File)
    foreach ($root in $remainingRoots) {
        $knownUpstream = $false
        foreach ($library in $archives) {
            $libraryText = [Text.Encoding]::UTF8.GetString([IO.File]::ReadAllBytes($library.FullName))
            if ($libraryText.Contains($root)) { $knownUpstream = $true; break }
        }
        if (-not $knownUpstream) { throw 'An unrecognized user-shaped path remains in the binary.' }
    }

    $archiveName = "VoiceTrigger-v$version-windows-x86_64"
    $package = Join-Path $output $archiveName
    [IO.Directory]::CreateDirectory($package) | Out-Null
    Copy-Item -LiteralPath $binary -Destination (Join-Path $output 'voice-trigger.exe')
    Copy-Item -LiteralPath $binary -Destination (Join-Path $package 'voice-trigger.exe')
    Copy-Item -LiteralPath (Join-Path $workspace 'LICENSE') -Destination (Join-Path $package 'LICENSE')
    $readme = [IO.File]::ReadAllText((Join-Path $workspace 'resources/portable-README.txt')).Replace('@VERSION@', $version)
    [IO.File]::WriteAllText((Join-Path $package 'README.txt'), $readme, [Text.UTF8Encoding]::new($false))
    $publicDocs = @('architecture.md', 'chatgpt.md', 'debug.md', 'development.md', 'interface.md', 'privacy.md', 'usage.md', 'releasing.md', 'images/interface.png', 'images/interface-dark.png')
    foreach ($relative in $publicDocs) {
        $destination = Join-Path $package "doc/$relative"
        [IO.Directory]::CreateDirectory([IO.Path]::GetDirectoryName($destination)) | Out-Null
        Copy-Item -LiteralPath (Join-Path $workspace "doc/$relative") -Destination $destination
    }

    $notices = @('# Third-party notices', '', 'Original license files are included under licenses/. Build-time dependencies are listed as well.', '', '| Package | Version | License |', '| --- | --- | --- |')
    $resolvedIds = @($metadata.resolve.nodes | ForEach-Object { $_.id })
    foreach ($dependency in ($metadata.packages | Sort-Object name)) {
        if ($dependency.name -eq 'voice-trigger' -or $dependency.id -notin $resolvedIds) { continue }
        $folder = [IO.Path]::GetDirectoryName($dependency.manifest_path)
        $licenseFiles = @(Get-ChildItem -LiteralPath $folder -File | Where-Object { $_.Name -match '^(LICENSE|COPYING)' })
        if ($licenseFiles.Count -eq 0) { throw "License files missing for $($dependency.name)." }
        $licenseFolder = Join-Path $package "licenses/$($dependency.name)-$($dependency.version)"
        [IO.Directory]::CreateDirectory($licenseFolder) | Out-Null
        foreach ($file in $licenseFiles) { Copy-Item -LiteralPath $file.FullName -Destination (Join-Path $licenseFolder $file.Name) }
        $notices += "| $($dependency.name) | $($dependency.version) | $($dependency.license) |"
    }
    $rustVersion = ((& rustc --version) -split ' ')[1]
    $rustLicenses = Join-Path $package "licenses/rust-$rustVersion"
    [IO.Directory]::CreateDirectory($rustLicenses) | Out-Null
    foreach ($name in @('COPYRIGHT', 'LICENSE-MIT', 'LICENSE-APACHE')) {
        Copy-Item -LiteralPath (Join-Path $sysroot "share/doc/rust/$name") -Destination (Join-Path $rustLicenses $name)
    }
    $notices += "`nRust $rustVersion standard-library/toolchain notices are included under licenses/rust-$rustVersion/."
    [IO.File]::WriteAllLines((Join-Path $package 'THIRD-PARTY-NOTICES.md'), $notices, [Text.UTF8Encoding]::new($false))

    $zip = Join-Path $output "$archiveName.zip"
    [IO.Compression.ZipFile]::CreateFromDirectory($package, $zip, [IO.Compression.CompressionLevel]::Optimal, $false)
    $archive = [IO.Compression.ZipFile]::OpenRead($zip)
    try { $entries = @($archive.Entries | ForEach-Object { $_.FullName }) }
    finally { $archive.Dispose() }
    if ($entries -match '(?i)(^|/)(settings\.json|ForAi\.md|AGENTS\.md|\.git|\.codex|\.claude|tracking|target)(/|$)|\.(log|pdb|tmp)$') {
        throw 'An unexpected private/development artifact entered the archive.'
    }
    $sums = @()
    $hashes = [ordered]@{}
    foreach ($name in @('voice-trigger.exe', "$archiveName.zip")) {
        $hash = (Get-FileHash -LiteralPath (Join-Path $output $name) -Algorithm SHA256).Hash.ToLowerInvariant()
        $hashes[$name] = $hash
        $sums += "$hash  $name"
    }
    [IO.File]::WriteAllLines((Join-Path $output 'SHA256SUMS.txt'), $sums, [Text.UTF8Encoding]::new($false))
    [ordered]@{ version = $version; target = 'x86_64-pc-windows-msvc'; localBuildPathsAbsent = $true; verifiedUpstreamToolchainPathRoots = $remainingRoots.Count; hashes = $hashes; packageFiles = $entries } |
        ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $output 'build-receipt.json') -Encoding utf8
    Write-Output "Release assets prepared under $OutputDirectory."
} finally {
    $env:CARGO_ENCODED_RUSTFLAGS = $oldEncodedFlags
    Pop-Location
}
