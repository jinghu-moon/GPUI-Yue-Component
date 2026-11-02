param([switch]$Release, [switch]$Profile)
$ErrorActionPreference = 'Stop'
$repo = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$backend = Join-Path $repo 'vendor/gpui-pre-windows'
# The standalone vendor package does not inherit the consumer workspace patch.
# Carry the same lock and explicitly select the maintained core in this test.
Copy-Item -LiteralPath (Join-Path $repo 'Cargo.lock') -Destination (Join-Path $backend 'Cargo.lock') -Force
$core = (Join-Path $repo 'vendor/gpui-pre').Replace('\', '/')
$filter = if ($Profile) { 'directx_renderer::backdrop::tests::profile_hardware' } else { 'directx_renderer::backdrop::tests' }
$cargoArgs = @('test', '--manifest-path', (Join-Path $backend 'Cargo.toml'), '--target-dir', (Join-Path $repo 'target'), '--lib', $filter, '--features', 'test-support', '--offline', '--config', "patch.crates-io.gpui-pre.path='$core'", '--config', 'profile.dev.debug=0', '--config', 'profile.test.debug=0')
if ($Profile) {
    $Release = $true
    # The benchmark installs its own profiler after pipeline construction.
    # An inherited live-diagnostics setting must not skew cold setup timings.
    $env:GPUI_BACKDROP_PROFILE = '0'
}
if ($Release) { $cargoArgs += '--release' }
$cargoArgs += @('--', '--nocapture')
if ($Profile) { $cargoArgs += @('--ignored', '--exact', '--test-threads=1') }
& cargo @cargoArgs
exit $LASTEXITCODE
