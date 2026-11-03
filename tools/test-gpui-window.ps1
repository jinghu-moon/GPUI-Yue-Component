param([switch]$Native, [switch]$Release, [string]$CorePath)
$ErrorActionPreference = 'Stop'
$repo = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$backend = Join-Path $repo 'vendor/gpui-pre-windows'
if (-not $CorePath) { $CorePath = Join-Path $repo 'vendor/gpui-pre' }
$core = (Resolve-Path -LiteralPath $CorePath).Path.Replace('\', '/')
Copy-Item -LiteralPath (Join-Path $repo 'Cargo.lock') -Destination (Join-Path $backend 'Cargo.lock') -Force
$filters = if ($Native) { @(
    'window::activation_tests::native_nonactivating_window_never_steals_focus',
    'window::activation_tests::native_physical_size_survives_dpi_changes'
) } else { @('window::activation_tests') }
# A native Application owns process-global Win32 class/dispatcher state. Start
# each native probe in its own test process rather than constructing two Apps
# sequentially inside one Rust test runner.
foreach ($filter in $filters) {
    $cargoArgs = @('test', '--manifest-path', (Join-Path $backend 'Cargo.toml'), '--target-dir', (Join-Path $repo 'target'), '--lib', $filter, '--features', 'test-support', '--offline', '--config', "patch.crates-io.gpui-pre.path='$core'", '--config', 'profile.dev.debug=0', '--config', 'profile.test.debug=0')
    if ($Release) { $cargoArgs += '--release' }
    $cargoArgs += @('--', '--nocapture', '--test-threads=1')
    if ($Native) { $cargoArgs += @('--ignored', '--exact') }
    & cargo @cargoArgs
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}
exit 0
