param([string]$Version = '0.1.0')
$ErrorActionPreference = 'Stop'
Set-Location (Split-Path $PSScriptRoot -Parent)
& "$PSScriptRoot/verify-version.ps1" -Version $Version
if (!(Test-Path .deps/ghboost-core/Cargo.toml)) { throw '请先运行 scripts/prepare-core.ps1。' }
npm ci
if ($LASTEXITCODE -ne 0) { throw 'npm ci failed' }
cargo test --manifest-path src-tauri/Cargo.toml --no-default-features --lib --locked
if ($LASTEXITCODE -ne 0) { throw 'cargo test failed' }
cargo build --manifest-path src-tauri/Cargo.toml --no-default-features --bin gharchive --release --locked
if ($LASTEXITCODE -ne 0) { throw 'CLI build failed' }
npm run tauri -- build --bundles nsis,msi -- --locked
if ($LASTEXITCODE -ne 0) { throw 'Tauri build failed' }
Write-Host 'CLI: src-tauri/target/release/gharchive.exe'
Write-Host 'GUI: src-tauri/target/release/gharchive-desktop.exe'
Write-Host 'Installers: src-tauri/target/release/bundle/'
