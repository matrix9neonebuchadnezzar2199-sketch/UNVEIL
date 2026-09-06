#Requires -Version 5.1
Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

Write-Host "== cargo fmt --check =="
cargo fmt --all -- --check

Write-Host "== cargo clippy (exclude unveil) =="
cargo clippy --workspace --exclude unveil --all-targets -- -D warnings

Write-Host "== cargo test (exclude unveil) =="
cargo test --workspace --exclude unveil

Write-Host "== python TEST.py =="
python TEST.py
if ($LASTEXITCODE -ne 0) {
    throw "TEST.py failed"
}

Write-Host "check.ps1 ok"
