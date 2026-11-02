param(
    [string]$Filter = 'toast::tests',
    [switch]$Offline
)
$ErrorActionPreference = 'Stop'
$repo = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$fixture = Join-Path $repo 'target/gpui-base-contract'
New-Item -ItemType Directory -Force $fixture | Out-Null

# The published Base manifest has unconditional development dependencies for
# both backends. Test our maintained source with the same single backend Yue
# uses, without editing that manifest or the Cargo registry.
$manifest = [IO.File]::ReadAllText((Join-Path $repo 'vendor/gpui-base/Cargo.toml'))
$manifest = [regex]::Replace($manifest, '(?ms)^\[dev-dependencies\.gpui-fast(?:-platform)?\][\s\S]*?(?=^\[|\z)', '')
$source = (Join-Path $repo 'vendor/gpui-base/src/lib.rs').Replace('\', '/')
$manifest = $manifest.Replace('path = "src/lib.rs"', ('path = "' + $source + '"'))
$manifest += "`n[workspace]`n"
$backend = (Join-Path $repo 'vendor/gpui-pre').Replace('\', '/')
$manifest += "`n[patch.crates-io]`ngpui-pre = { path = `"$backend`" }`n"
[IO.File]::WriteAllText((Join-Path $fixture 'Cargo.toml'), $manifest, [Text.UTF8Encoding]::new($false))
# Preserve the consuming workspace's production pins; Cargo adds test-only deps
# to this disposable lockfile. It never changes the workspace's Cargo.lock.
Copy-Item -LiteralPath (Join-Path $repo 'Cargo.lock') -Destination (Join-Path $fixture 'Cargo.lock') -Force
$cargoArgs = @('test', '--manifest-path', (Join-Path $fixture 'Cargo.toml'), '--target-dir', (Join-Path $repo 'target'), '--features', 'test-support', '--lib', $Filter)
if ($Offline) { $cargoArgs += '--offline' }
& cargo @cargoArgs
exit $LASTEXITCODE
