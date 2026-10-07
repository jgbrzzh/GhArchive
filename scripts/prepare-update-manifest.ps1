param([Parameter(Mandatory)][string]$Version, [string]$OutputDirectory = 'artifacts')
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
& "$PSScriptRoot/verify-version.ps1" -Version $Version
$Version = $Version -replace '^v',''
$out = if ([IO.Path]::IsPathRooted($OutputDirectory)) { $OutputDirectory } else { Join-Path $root $OutputDirectory }
New-Item -ItemType Directory -Force $out | Out-Null
$platforms = @{}
foreach ($kind in @('nsis','msi')) {
    $extension = if ($kind -eq 'nsis') { '.exe' } else { '.msi' }
    $files = @(Get-ChildItem (Join-Path $root "src-tauri/target/release/bundle/$kind") -File | Where-Object { $_.Extension -eq $extension -and $_.Name -like "GhArchive_$($Version)_*" })
    if ($files.Count -ne 1) { throw "需要唯一的 $kind 安装包，版本 $Version" }
    $file = $files[0]
    $signaturePath = $file.FullName + '.sig'
    if (!(Test-Path -LiteralPath $signaturePath)) { throw "缺少更新签名：$($file.Name).sig" }
    $signature = (Get-Content -LiteralPath $signaturePath -Raw -Encoding UTF8).Trim()
    if ([string]::IsNullOrEmpty($signature)) { throw '更新签名不能为空' }
    $entry = @{ url = "https://github.com/jgbrzzh/GhArchive/releases/download/v$Version/$($file.Name)"; signature = $signature }
    $platforms["windows-x86_64-$kind"] = $entry
    if ($kind -eq 'nsis') { $platforms['windows-x86_64'] = $entry }
    Copy-Item -LiteralPath $file.FullName,$signaturePath -Destination $out -Force
}
$manifest = @{ version=$Version; notes="GhArchive $Version。完整更新说明见 GitHub Release。"; pub_date=[DateTime]::UtcNow.ToString('yyyy-MM-ddTHH:mm:ssZ'); platforms=$platforms }
$json = $manifest | ConvertTo-Json -Depth 6
[IO.File]::WriteAllText((Join-Path $out 'latest.json'), $json, (New-Object Text.UTF8Encoding($false)))
Write-Host "Signed update manifest prepared: $out/latest.json"
