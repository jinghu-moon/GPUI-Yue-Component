param([switch]$Offline)
$ErrorActionPreference = 'Stop'
$repo = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$fixture = Join-Path $repo 'target/gpui-snapshot-contract'
New-Item -ItemType Directory -Force $fixture | Out-Null
$source = (Join-Path $repo 'vendor/gpui-pre').Replace('\', '/')
$tests = (Join-Path $source 'tests/paint_snapshot.rs').Replace('\', '/')
$backdropTests = (Join-Path $source 'tests/backdrop_blur.rs').Replace('\', '/')
$manifest = @"
[package]
name = "yue-gpui-snapshot-contract"
version = "0.1.0"
edition = "2024"
[workspace]
[[test]]
name = "paint_snapshot"
path = "$tests"
[[test]]
name = "backdrop_blur"
path = "$backdropTests"
[dependencies]
gpui = { package = "gpui-pre", path = "$source", features = ["test-support"] }
image = { version = "0.25", default-features = false }
"@
[IO.File]::WriteAllText((Join-Path $fixture 'Cargo.toml'), $manifest, [Text.UTF8Encoding]::new($false))
Copy-Item -LiteralPath (Join-Path $repo 'Cargo.lock') -Destination (Join-Path $fixture 'Cargo.lock') -Force
$cargoArgs = @('test', '--manifest-path', (Join-Path $fixture 'Cargo.toml'), '--target-dir', (Join-Path $repo 'target'), '--test', 'paint_snapshot', '--test', 'backdrop_blur')
if ($Offline) { $cargoArgs += '--offline' }
& cargo @cargoArgs
exit $LASTEXITCODE
