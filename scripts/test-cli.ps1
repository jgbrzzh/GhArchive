param([string]$Exe = 'src-tauri/target/debug/gharchive.exe')
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
Set-Location $root
$Exe = (Resolve-Path $Exe).Path
$previous = $env:GHARCHIVE_DATA_DIR
$testDir = Join-Path $root ('output/cli-smoke-' + [guid]::NewGuid().ToString('N'))
$env:GHARCHIVE_DATA_DIR = Join-Path $testDir 'data'
function Invoke-Json([string[]]$CommandArgs, [int]$ExpectedExit = 0) {
    $output = & $Exe @CommandArgs --json
    $code = $LASTEXITCODE
    if ($code -ne $ExpectedExit) { throw "Unexpected exit $code for $($CommandArgs -join ' '): $output" }
    $r = ($output -join "`n") | ConvertFrom-Json
    if ($null -eq $r.success -or !$r.timestamp) { throw 'Invalid envelope' }
    if ($ExpectedExit -eq 0 -and !$r.success) { throw 'Command returned failure' }
    return $r
}
try {
    $null = Invoke-Json @('--help')
    $null = Invoke-Json @('describe')
    $null = Invoke-Json @('unknown') 1
    $null = Invoke-Json @('config','set','concurrency','2')
    $null = Invoke-Json @('config','set','concurrency','0') 1
    $null = Invoke-Json @('config','get')
    $null = Invoke-Json @('config','get','unknown') 4
    $backup = Join-Path $testDir 'backups'
    $task = Invoke-Json @('add','octocat/git-consortium','--time','03:00','--dir',$backup,'--name','Smoke','--disabled')
    $id = [string]$task.data.id
    $null = Invoke-Json @('list')
    $null = Invoke-Json @('enable',$id)
    $null = Invoke-Json @('disable',$id)
    $null = Invoke-Json @('run',$id) 2
    $h = Invoke-Json @('history','--limit','20')
    if ($h.data[0].status -ne 'failed') { throw 'Failed preflight not recorded' }
    $null = Invoke-Json @('status')
    $null = Invoke-Json @('run-all')
    $null = Invoke-Json @('remove',$id) 2
    $null = Invoke-Json @('remove',$id,'--yes')
    $null = Invoke-Json @('run',$id) 4
    Write-Host "CLI checks passed; isolated evidence: $testDir"
} finally {
    $env:GHARCHIVE_DATA_DIR = $previous
}
