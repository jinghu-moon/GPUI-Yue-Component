param(
    [string]$TargetDir,
    [switch]$Release,
    [switch]$Offline,
    [switch]$IncludeFrameProfile
)
$ErrorActionPreference = 'Stop'
$repo = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
if (-not $TargetDir) { $TargetDir = Join-Path $repo 'target' }
$logDir = Join-Path $TargetDir ('virtualization-native/' + (Get-Date -Format 'yyyyMMdd-HHmmss'))
New-Item -ItemType Directory -Path $logDir -Force | Out-Null
# Native Application owns process-global Win32 state. Keep each probe in its
# own process when this list grows; do not combine filters into one test run.
$filters = @(
    'pages::virtual_layouts::native_acceptance::native_resize_visibility_resume',
    'pages::virtual_layouts::native_tests::native_frozen_grid_and_masonry_cache_restore',
    'pages::virtual_layouts::native_tests::native_layouts_render_and_settle',
    'pages::virtual_layouts::native_tests::native_smooth_layouts_sample_and_settle'
)
if ($IncludeFrameProfile) { $filters += 'pages::virtual_layouts::native_profile::native_frame_workload' }
foreach ($filter in $filters) {
    $cargoArgs = @('test', '--manifest-path', (Join-Path $repo 'Cargo.toml'), '-p', 'windows-showcase', '--bin', 'windows-showcase', '--locked', '--target-dir', $TargetDir)
    if ($Release) { $cargoArgs += '--release' }
    if ($Offline) { $cargoArgs += '--offline' }
    if ($IncludeFrameProfile) { $cargoArgs += @('--features', 'virtual-profile') }
    $cargoArgs += @('--', '--ignored', '--exact', $filter, '--nocapture', '--test-threads=1')
    # Cargo's progress uses stderr. Capture it without PowerShell converting
    # ordinary progress into a terminating NativeCommandError on older hosts.
    $previousPreference = $ErrorActionPreference
    try {
        $ErrorActionPreference = 'Continue'
        $lines = @(& cargo @cargoArgs 2>&1)
        $cargoExit = $LASTEXITCODE
    } finally { $ErrorActionPreference = $previousPreference }
    $lines | ForEach-Object { Write-Host $_ }
    $log = Join-Path $logDir (($filter -replace '::', '-') + '.log')
    [IO.File]::WriteAllLines($log, [string[]]$lines, [Text.UTF8Encoding]::new($false))
    if ($cargoExit -ne 0) { exit $cargoExit }
    & python (Join-Path $PSScriptRoot 'virtualization_evidence.py') native --log $log --filter $filter
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}
exit 0
