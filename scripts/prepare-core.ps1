param([string]$GhBoostPath)
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
$lock = Get-Content (Join-Path $root 'ghboost-core.lock.json') -Encoding UTF8 -Raw | ConvertFrom-Json
$destination = Join-Path $root '.deps/ghboost-core'
if (!$GhBoostPath) { $GhBoostPath = Join-Path $root '.deps/ghboost-source' }
$source = Join-Path $GhBoostPath $lock.path
if (!(Test-Path (Join-Path $source 'Cargo.toml'))) { throw '核心文件不存在。使用 -GhBoostPath 指定 GhBoost checkout；源码只复制到忽略的 .deps 目录。' }
$revision = (& git -C $GhBoostPath rev-parse HEAD).Trim()
if ($LASTEXITCODE -ne 0 -or $revision -ne $lock.revision) { throw "GhBoost 版本不匹配，要求 $($lock.revision)。请使用该提交的独立 checkout。" }
$changes = & git -C $GhBoostPath status --porcelain -- $lock.path
if ($LASTEXITCODE -ne 0 -or $changes) { throw '核心目录存在未提交修改。请使用固定提交的干净 checkout，避免本地构建与 Actions 使用不同源码。' }
New-Item -ItemType Directory -Force $destination | Out-Null
Copy-Item (Join-Path $source 'Cargo.toml'),(Join-Path $source 'README.md') -Destination $destination -Force
foreach ($folder in @('src','migrations')) { Copy-Item (Join-Path $source $folder) -Destination $destination -Recurse -Force }
Write-Host "Private core prepared: $($lock.repository)@$revision -> .deps/ghboost-core"
