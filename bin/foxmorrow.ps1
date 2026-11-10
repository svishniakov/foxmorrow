$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$env:FOXMORROW_PLUGIN_ROOT = $root

function Fail([string] $message) {
    [Console]::Error.WriteLine("foxmorrow: $message")
    exit 2
}

if ($env:FOXMORROW_DEV_BINARY) {
    & $env:FOXMORROW_DEV_BINARY @args
    exit $LASTEXITCODE
}

$checksumsPath = Join-Path $root 'checksums.json'
if (-not (Test-Path $checksumsPath)) { Fail "missing $checksumsPath" }
$checksums = Get-Content -Raw $checksumsPath | ConvertFrom-Json
if ($env:PROCESSOR_ARCHITECTURE -ne 'AMD64') { Fail "unsupported platform Windows $($env:PROCESSOR_ARCHITECTURE)" }
$target = 'x86_64-pc-windows-msvc'
$version = $checksums.version
$expected = $checksums.binaries.$target
if (-not $expected) { Fail "no released binary for $target in version $version; run devin plugins update or wait for the release" }

$cacheRoot = if ($env:LOCALAPPDATA) { $env:LOCALAPPDATA } else { Join-Path $HOME '.cache' }
$cache = Join-Path $cacheRoot "foxmorrow\$version\$target"
$binary = Join-Path $cache 'foxmorrow.exe'
if (-not (Test-Path $binary)) {
    New-Item -ItemType Directory -Force -Path $cache | Out-Null
    $partial = Join-Path $cache "foxmorrow.download.$PID.exe"
    $base = if ($env:FOXMORROW_RELEASE_BASE) { $env:FOXMORROW_RELEASE_BASE } else { "https://github.com/$($checksums.repository)/releases/download" }
    $url = "$base/v$version/foxmorrow-$target.exe"
    try {
        (New-Object System.Net.WebClient).DownloadFile($url, $partial)
    } catch {
        Remove-Item -ErrorAction SilentlyContinue $partial
        Fail "download failed: $url"
    }
    $stream = [System.IO.File]::OpenRead($partial)
    try {
        $digest = [System.Security.Cryptography.SHA256]::Create().ComputeHash($stream)
    } finally {
        $stream.Dispose()
    }
    $actual = -join ($digest | ForEach-Object { $_.ToString('x2') })
    if ($actual -ne $expected) {
        Remove-Item -ErrorAction SilentlyContinue $partial
        Fail "checksum mismatch for ${url}: expected $expected, got $actual"
    }
    Move-Item -Force $partial $binary
}

& $binary @args
exit $LASTEXITCODE
