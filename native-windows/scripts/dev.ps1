$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
cargo run --manifest-path (Join-Path $root "Cargo.toml")
