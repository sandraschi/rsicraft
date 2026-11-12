# Model-swap benchmark: same harness, same tasks, different models.
#
# For every model x task x trial it runs a fresh campaign in its own database
# and appends one row to results.csv. Resumable: rows already in the CSV are
# skipped, so the script can be restarted after an interruption.
#
# The per-run databases are kept. They hold the full lineage of every search
# and are the raw material for trace export later.
#
# Example:
#   pwsh -File scripts\bench-models.ps1 -Models 'qwen3.6-35b-a3b:latest','hf.co/FrontisAI/Frontis-MA1-35B-GGUF:Q4_K_M'
param(
    [string[]]$Models = @('qwen3.6-35b-a3b:latest', 'hf.co/FrontisAI/Frontis-MA1-35B-GGUF:Q4_K_M'),
    [string]$Pack = (Join-Path $PSScriptRoot '..\tasks\pack-v1'),
    [string[]]$OnlyTasks = @(),
    [int]$Steps = 12,
    [int]$Trials = 3,
    [int]$TimeoutSecs = 60,
    [string]$LlamaUrl = 'http://127.0.0.1:11434',
    [string]$OutDir = '',
    [string]$Exe = (Join-Path $PSScriptRoot '..\target\release\rsicraft.exe')
)

$ErrorActionPreference = 'Continue'

# powershell -File passes 'a,b' as one string; accept that form too.
$Models = @($Models | ForEach-Object { $_ -split ',' } | ForEach-Object { $_.Trim() } | Where-Object { $_ })
$OnlyTasks = @($OnlyTasks | ForEach-Object { $_ -split ',' } | ForEach-Object { $_.Trim() } | Where-Object { $_ })

if (-not $OutDir) {
    $OutDir = Join-Path $env:LOCALAPPDATA ('rsicraft\bench\' + (Get-Date -Format 'yyyyMMdd-HHmm'))
}
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
if (-not (Test-Path -LiteralPath $Exe)) {
    Write-Host "[ERROR] $Exe not found. Run: cargo build --release -p rsicraft-mcp" -ForegroundColor Red
    exit 1
}
# Run from a private copy so later builds cannot replace the binary mid-run.
$RunExe = Join-Path $OutDir 'rsicraft.exe'
if (-not (Test-Path -LiteralPath $RunExe)) {
    Copy-Item -LiteralPath $Exe -Destination $RunExe
}

$Csv = Join-Path $OutDir 'results.csv'
$Log = Join-Path $OutDir 'bench.log'
if (-not (Test-Path -LiteralPath $Csv)) {
    'model,task,split,trial,steps_requested,steps_executed,steps_skipped,aborted,success_cards,best_score,first_success_step,seconds' |
        Set-Content -LiteralPath $Csv -Encoding ascii
}

function Write-Log([string]$Text) {
    $line = '{0} {1}' -f (Get-Date -Format 'HH:mm:ss'), $Text
    Add-Content -LiteralPath $Log -Value $line -Encoding ascii
    Write-Host $line
}

function Read-Task([string]$Path) {
    $lines = Get-Content -LiteralPath $Path
    $meta = @{}
    foreach ($l in $lines) {
        if ($l -match '^# (id|name|split|description): (.*)$') { $meta[$Matches[1]] = $Matches[2].Trim() }
        elseif ($l -notmatch '^#') { break }
    }
    $meta['evaluator'] = ($lines -join "`n") + "`n"
    return $meta
}

$tasks = Get-ChildItem -LiteralPath $Pack -Filter '*.py' | Sort-Object Name | ForEach-Object { Read-Task $_.FullName }
if ($OnlyTasks.Count -gt 0) { $tasks = $tasks | Where-Object { $OnlyTasks -contains $_['id'] } }
$done = @{}
Import-Csv -LiteralPath $Csv | ForEach-Object { $done["$($_.model)|$($_.task)|$($_.trial)"] = $true }

Write-Log "bench start: models=$($Models -join ';') tasks=$($tasks.Count) trials=$Trials steps=$Steps out=$OutDir"

foreach ($model in $Models) {
    $safeModel = ($model -replace '[^A-Za-z0-9._-]', '_')
    $modelDir = Join-Path $OutDir $safeModel
    New-Item -ItemType Directory -Force -Path $modelDir | Out-Null
    foreach ($trial in 1..$Trials) {
        foreach ($task in $tasks) {
            $id = $task['id']
            if ($done["$model|$id|$trial"]) { continue }
            $db = Join-Path $modelDir "$id-t$trial.db"
            Remove-Item -LiteralPath $db, "$db-wal", "$db-shm" -ErrorAction SilentlyContinue

            $upsert = @{ jsonrpc = '2.0'; id = 1; method = 'tools/call'; params = @{ name = 'rsi_task_upsert'; arguments = @{
                        task_id = $id; name = $task['name']; description = $task['description']; evaluator_script = $task['evaluator'] } } } |
                ConvertTo-Json -Compress -Depth 8
            $null = $upsert | & $RunExe --db $db mcp 2>$null

            $sw = [Diagnostics.Stopwatch]::StartNew()
            $raw = & $RunExe --db $db --llama-url $LlamaUrl --model $model --json campaign --task-id $id --steps $Steps --provider llama --sandbox-mode local --timeout-secs $TimeoutSecs 2>$null
            $secs = [int]$sw.Elapsed.TotalSeconds
            $res = $null
            try { $res = ($raw -join "`n") | ConvertFrom-Json } catch { $res = $null }

            if ($null -eq $res -or $null -eq $res.steps) {
                $err = if ($res) { [string]$res.error } else { 'no parsable output' }
                Write-Log "FAILED $model $id t$trial after ${secs}s: $err"
                $row = '{0},{1},{2},{3},{4},0,0,True,0,0,,{5}' -f $model, $id, $task['split'], $trial, $Steps, $secs
            }
            else {
                $n = 0; $first = ''; $ok = 0
                foreach ($s in $res.steps) {
                    $n++
                    if ($s.status -eq 'SUCCESS') { $ok++; if ($first -eq '') { $first = $n } }
                }
                $row = '{0},{1},{2},{3},{4},{5},{6},{7},{8},{9},{10},{11}' -f $model, $id, $task['split'], $trial, $Steps,
                    $res.steps_executed, $res.steps_skipped, $res.aborted, $ok, ([double]$res.best_score).ToString([cultureinfo]::InvariantCulture), $first, $secs
                Write-Log "done $model $id t$trial best=$($res.best_score) ok=$ok/$($res.steps_executed) skipped=$($res.steps_skipped) ${secs}s"
            }
            Add-Content -LiteralPath $Csv -Value $row -Encoding ascii
        }
    }
}
Write-Log 'bench complete'
