$ErrorActionPreference = 'Stop'
Set-Location $PSScriptRoot
if (!(Test-Path '.deps/ghboost-core/Cargo.toml')) { throw '先运行 scripts/prepare-core.ps1 -GhBoostPath <GhBoost目录>。' }
if (!(Test-Path 'node_modules/.bin/tauri.cmd')) {
    npm ci
    if ($LASTEXITCODE -ne 0) { throw 'npm ci failed' }
}
# Tauri 启动本项目独立的 1427 端口；端口占用时 Vite strictPort 会明确失败。
npm run desktop
exit $LASTEXITCODE
