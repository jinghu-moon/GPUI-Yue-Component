param(
    [string]$TargetDir,
    [switch]$Release,
    [switch]$Offline
)
$ErrorActionPreference = 'Stop'
$repo = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
if (-not $TargetDir) { $TargetDir = Join-Path $repo 'target' }
# Native Application owns process-global Win32 state. Keep each probe in its
# own process when this list grows; do not combine filters into one test run.
$filters = @(
    'pages::virtual_layouts::native_tests::native_frozen_grid_and_masonry_cache_restore',
    'pages::virtual_layouts::native_tests::native_layouts_render_and_settle',
    'pages::virtual_layouts::native_tests::native_smooth_layouts_sample_and_settle'
)
foreach ($filter in $filters) {
    $cargoArgs = @('test', '--manifest-path', (Join-Path $repo 'Cargo.toml'), '-p', 'windows-showcase', '--bin', 'windows-showcase', '--locked', '--target-dir', $TargetDir)
    if ($Release) { $cargoArgs += '--release' }
    if ($Offline) { $cargoArgs += '--offline' }
    $cargoArgs += @('--', '--ignored', '--exact', $filter, '--nocapture', '--test-threads=1')
    & cargo @cargoArgs
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}
exit 0
