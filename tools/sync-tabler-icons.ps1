[CmdletBinding(DefaultParameterSetName = 'Latest')]
param(
    [Parameter(ParameterSetName = 'Version')]
    [string]$Version,
    [Parameter(ParameterSetName = 'Latest')]
    [switch]$Latest,
    [Parameter(ParameterSetName = 'Check')]
    [switch]$Check
)

$ErrorActionPreference = 'Stop'
$repository = 'https://github.com/tabler/tabler-icons'
$api = 'https://api.github.com/repos/tabler/tabler-icons'
$root = Split-Path -Parent $PSScriptRoot
$crate = Join-Path $root 'crates/yue-component-icons'
$assets = Join-Path $crate 'assets/tabler'
$lockPath = Join-Path $crate 'tabler-icons-version.json'
$headers = @{ 'User-Agent' = 'GPUI-Yue-Component icon sync' }

if ($PSCmdlet.ParameterSetName -eq 'Version') {
    $tag = $Version
    if (-not $tag.StartsWith('v')) { $tag = "v$tag" }
} else {
    $release = Invoke-RestMethod -Headers $headers -Uri "$api/releases/latest"
    $tag = $release.tag_name
}

$remote = @(git ls-remote --tags --refs $repository "refs/tags/$tag")
if ($LASTEXITCODE -ne 0 -or $remote.Count -eq 0) {
    throw "Could not resolve official Tabler tag $tag"
}
$commit = ($remote[0] -split '\s+')[0]
$versionNumber = $tag.TrimStart('v')

if ($PSCmdlet.ParameterSetName -eq 'Check') {
    $current = if (Test-Path $lockPath) { Get-Content -Raw $lockPath | ConvertFrom-Json } else { $null }
    if ($current -and $current.tag -eq $tag) {
        Write-Output "Tabler icons are current at $tag ($commit)."
        exit 0
    }
    $currentTag = if ($current) { $current.tag } else { 'not synced' }
    Write-Output "Tabler update available: $currentTag -> $tag ($commit)."
    exit 1
}

$tempRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("yue-tabler-" + [guid]::NewGuid().ToString('N'))
$archive = Join-Path $tempRoot 'tabler.tar.gz'
$extract = Join-Path $tempRoot 'extract'
New-Item -ItemType Directory -Path $extract -Force | Out-Null
try {
    Invoke-WebRequest -Headers $headers -Uri "https://codeload.github.com/tabler/tabler-icons/tar.gz/refs/tags/$tag" -OutFile $archive
    tar -xzf $archive -C $extract
    if ($LASTEXITCODE -ne 0) { throw "Failed to extract Tabler archive for $tag" }
    $source = Get-ChildItem -Directory $extract | Select-Object -First 1
    if (-not $source) { throw "Tabler archive for $tag is empty" }
    foreach ($style in @('outline', 'filled')) {
        $sourceDir = Join-Path $source.FullName "icons/$style"
        if (-not (Test-Path $sourceDir)) { throw "Missing official icons/$style at $tag" }
    }

    $licenseSource = Join-Path $source.FullName 'LICENSE'
    if (-not (Test-Path $licenseSource)) { throw "Missing official LICENSE at $tag" }
    if (Test-Path $assets) {
        $resolvedAssets = (Resolve-Path $assets).Path
        $expectedAssets = Join-Path (Resolve-Path $crate).Path 'assets/tabler'
        if (-not [string]::Equals($resolvedAssets, $expectedAssets, [StringComparison]::OrdinalIgnoreCase)) {
            throw "Refusing to replace assets outside $crate"
        }
        Remove-Item -LiteralPath $assets -Recurse -Force
    }
    New-Item -ItemType Directory -Path $assets -Force | Out-Null
    foreach ($style in @('outline', 'filled')) {
        Copy-Item -Path (Join-Path $source.FullName "icons/$style") -Destination $assets -Recurse
    }
    Copy-Item -LiteralPath $licenseSource -Destination (Join-Path $crate 'LICENSE.tabler') -Force

    $lock = [ordered]@{
        repository = $repository
        tag = $tag
        version = $versionNumber
        commit = $commit
        license = 'MIT'
    }
    $lock | ConvertTo-Json | Set-Content -LiteralPath $lockPath -Encoding utf8NoBOM
    $outlineCount = (Get-ChildItem (Join-Path $assets 'outline') -Filter '*.svg' -File).Count
    $filledCount = (Get-ChildItem (Join-Path $assets 'filled') -Filter '*.svg' -File).Count
    Write-Output "Synced Tabler $tag ($commit): $outlineCount outline, $filledCount filled icons."
} finally {
    if (Test-Path $tempRoot) { Remove-Item -LiteralPath $tempRoot -Recurse -Force }
}
