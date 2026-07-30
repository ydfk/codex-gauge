$ErrorActionPreference = "Stop"
$manifest = Join-Path (Split-Path -Parent $PSScriptRoot) "Cargo.toml"
cargo fmt --manifest-path $manifest --check
cargo check --manifest-path $manifest --locked
cargo test --manifest-path $manifest --locked
