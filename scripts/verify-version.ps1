param([Parameter(Mandatory)][string]$Version)
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
$Version = $Version -replace '^v',''
if ($Version -notmatch '^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?$') { throw '请明确指定语义版本，例如 0.1.0 或 v0.1.0。' }
$package = Get-Content (Join-Path $root 'package.json') -Encoding UTF8 -Raw | ConvertFrom-Json
$tauri = Get-Content (Join-Path $root 'src-tauri/tauri.conf.json') -Encoding UTF8 -Raw | ConvertFrom-Json
$cargo = Get-Content (Join-Path $root 'src-tauri/Cargo.toml') -Encoding UTF8 -Raw
$cargoVersion = [regex]::Match($cargo, '(?m)^version\s*=\s*"([^"]+)"').Groups[1].Value
if ($Version -ne $package.version -or $Version -ne $tauri.version -or $Version -ne $cargoVersion) { throw '请求版本与 package.json、Cargo.toml、tauri.conf.json 不一致。' }
Write-Host "Explicit version verified: $Version"
