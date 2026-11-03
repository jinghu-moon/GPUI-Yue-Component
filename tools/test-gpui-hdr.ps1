param([switch]$Release, [switch]$Native)
$ErrorActionPreference = 'Stop'
$repo = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$backend = Join-Path $repo 'vendor/gpui-pre-windows'
Copy-Item -LiteralPath (Join-Path $repo 'Cargo.lock') -Destination (Join-Path $backend 'Cargo.lock') -Force
$core = (Join-Path $repo 'vendor/gpui-pre').Replace('\', '/')
$filter = if ($Native) { 'native_hidden_color_output_resize_and_sdr_readback_restore_resources' } else { 'directx_' }
$cargoArgs = @('test', '--manifest-path', (Join-Path $backend 'Cargo.toml'), '--target-dir', (Join-Path $repo 'target'), '--lib', $filter, '--features', 'test-support', '--offline', '--config', "patch.crates-io.gpui-pre.path='$core'", '--config', 'profile.dev.debug=0', '--config', 'profile.test.debug=0')
if ($Release) { $cargoArgs += '--release' }
$cargoArgs += @('--', '--nocapture', '--test-threads=1')
if ($Native) { $cargoArgs += '--ignored' }
& cargo @cargoArgs
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
if ($Native) { exit 0 }
$policyArgs = $cargoArgs | ForEach-Object { if ($_ -eq 'directx_') { 'color_output::tests' } else { $_ } }
& cargo @policyArgs
exit $LASTEXITCODE
