param([string]$GhBoostPath)
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
$lock = Get-Content (Join-Path $root 'ghboost-core.lock.json') -Encoding UTF8 -Raw | ConvertFrom-Json
$destination = Join-Path $root '.deps/ghboost-core'
if (!$GhBoostPath) { $GhBoostPath = Join-Path $root '.deps/ghboost-source' }
$revision = $lock.revision
if ($revision -notmatch '^[0-9a-f]{40}$') { throw '核心锁定提交必须为完整 SHA。' }
& git -C $GhBoostPath cat-file -e "$revision^{commit}"
if ($LASTEXITCODE -ne 0) { throw "本地没有固定提交 $revision。请获取该提交后重试；脚本不会修改 GhBoost。" }
# Export committed objects, never the working tree of the actively maintained core.
$export = Join-Path $root ('.deps/core-export-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Force $export | Out-Null
try {
    $archive = Join-Path $export 'source.tar'
    & git -C $GhBoostPath archive --format=tar --output $archive $revision -- $lock.path
    if ($LASTEXITCODE -ne 0) { throw '无法导出固定提交的核心。' }
    & tar -xf $archive -C $export
    if ($LASTEXITCODE -ne 0) { throw '无法解包核心快照。' }
    $source = Join-Path $export $lock.path
    if (!(Test-Path (Join-Path $source 'Cargo.toml'))) { throw '该提交没有核心 Cargo.toml。' }
    New-Item -ItemType Directory -Force $destination | Out-Null
    Copy-Item (Join-Path $source 'Cargo.toml'),(Join-Path $source 'README.md') -Destination $destination -Force
    foreach ($folder in @('src','migrations','tests')) {
        $sourceFolder = Join-Path $source $folder
        $targetFolder = Join-Path $destination $folder
        if (Test-Path $targetFolder) {
            Get-ChildItem -LiteralPath $targetFolder -File -Recurse | ForEach-Object {
                $relative = $_.FullName.Substring($targetFolder.Length + 1)
                if (!(Test-Path -LiteralPath (Join-Path $sourceFolder $relative))) { Remove-Item -LiteralPath $_.FullName -Force }
            }
        }
        if (Test-Path $sourceFolder) { Copy-Item $sourceFolder -Destination $destination -Recurse -Force }
    }
    Write-Host "Private core prepared: $($lock.repository)@$revision -> .deps/ghboost-core"
} finally {
    $resolvedExport = (Resolve-Path -LiteralPath $export).Path
    $depsPrefix = [IO.Path]::GetFullPath((Join-Path $root '.deps')) + [IO.Path]::DirectorySeparatorChar
    if (!$resolvedExport.StartsWith($depsPrefix, [StringComparison]::OrdinalIgnoreCase)) { throw '临时导出目录不在 .deps，停止清理。' }
    Remove-Item -LiteralPath $resolvedExport -Recurse -Force
}
