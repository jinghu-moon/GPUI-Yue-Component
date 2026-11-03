param([switch]$Native, [switch]$Release, [string]$CorePath)
$ErrorActionPreference = 'Stop'
$repo = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$backend = Join-Path $repo 'vendor/gpui-pre-windows'
if (-not $CorePath) { $CorePath = Join-Path $repo 'vendor/gpui-pre' }
$core = (Resolve-Path -LiteralPath $CorePath).Path.Replace('\', '/')
Copy-Item -LiteralPath (Join-Path $repo 'Cargo.lock') -Destination (Join-Path $backend 'Cargo.lock') -Force
$filter = if ($Native) { 'window::activation_tests::native_nonactivating_window_never_steals_focus' } else { 'window::activation_tests' }
$cargoArgs = @('test', '--manifest-path', (Join-Path $backend 'Cargo.toml'), '--target-dir', (Join-Path $repo 'target'), '--lib', $filter, '--features', 'test-support', '--offline', '--config', "patch.crates-io.gpui-pre.path='$core'", '--config', 'profile.dev.debug=0', '--config', 'profile.test.debug=0')
if ($Release) { $cargoArgs += '--release' }
$cargoArgs += @('--', '--nocapture', '--test-threads=1')
if ($Native) { $cargoArgs += @('--ignored', '--exact') }
& cargo @cargoArgs
exit $LASTEXITCODE
