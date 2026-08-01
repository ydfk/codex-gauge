$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$manifest = Join-Path $root "Cargo.toml"
$dist = Join-Path $root "dist"

cargo build --manifest-path $manifest --release --locked --bin codex-gauge-native
New-Item -ItemType Directory -Force $dist | Out-Null
Copy-Item (Join-Path $root "target\release\codex-gauge-native.exe") (Join-Path $dist "Codex.Gauge.Native_x64.portable.exe") -Force
Write-Host "Built $dist\Codex.Gauge.Native_x64.portable.exe"
