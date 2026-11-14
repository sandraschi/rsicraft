# Rewrite rsi-base-35b so its template and parameters match rsi-frontis-35b.
# The weight blobs stay the base model's. Without -Apply this only prints the
# Modelfile. The previous base template printed a literal ".Prompt }}" and
# stopped on <think>. -Apply uses `ollama create -f` because /api/create
# rejects a Modelfile string.
param(
    [string]$Base = 'rsi-base-35b',
    [string]$Frontis = 'rsi-frontis-35b',
    [string]$LlamaUrl = 'http://127.0.0.1:11434',
    [switch]$Apply
)

$ErrorActionPreference = 'Stop'

function Get-AliasShow([string]$Name) {
    $body = '{"name":"' + $Name + '"}'
    $uri = $LlamaUrl.TrimEnd('/') + '/api/show'
    return Invoke-RestMethod -Uri $uri -Method Post -Body $body -ContentType 'application/json'
}

$baseShow = Get-AliasShow $Base
$frontisShow = Get-AliasShow $Frontis
$from = New-Object System.Collections.Generic.List[string]
foreach ($line in ($baseShow.modelfile -split "`n")) {
    $trim = $line.Trim().TrimEnd("`r")
    if ($trim.StartsWith('FROM ')) {
        $from.Add($trim)
    }
}
if ($from.Count -lt 1) {
    Write-Host "[ERROR] $Base has no FROM line"
    exit 1
}

$lines = New-Object System.Collections.Generic.List[string]
foreach ($blob in $from) {
    $lines.Add($blob)
}
$lines.Add('TEMPLATE {{ .Prompt }}')
$lines.Add('PARAMETER num_ctx 16384')
$modelfile = ($lines.ToArray() -join "`n") + "`n"

Write-Host "--- $Base parameters ---"
Write-Host $baseShow.parameters
Write-Host "--- $Frontis parameters ---"
Write-Host $frontisShow.parameters
Write-Host '--- proposed base modelfile ---'
Write-Host $modelfile

if (-not $Apply) {
    Write-Host 'Dry run. Pass -Apply to write the alias.'
    exit 0
}

$mfPath = Join-Path ([System.IO.Path]::GetTempPath()) ('rsicraft-' + $Base + '.Modelfile')
$utf8 = New-Object System.Text.UTF8Encoding $false
[System.IO.File]::WriteAllText($mfPath, $modelfile, $utf8)
& ollama create $Base -f $mfPath
if ($LASTEXITCODE -ne 0) {
    Write-Host "[ERROR] ollama create exited $LASTEXITCODE"
    exit $LASTEXITCODE
}
$after = Get-AliasShow $Base
Write-Host "--- $Base parameters after ---"
Write-Host $after.parameters
if ($after.parameters -match 'temperature' -or $after.parameters -match 'stop') {
    Write-Host '[ERROR] base alias still has stop or temperature parameters'
    exit 1
}
Write-Host "[ok] $Base now matches $Frontis on template and num_ctx"
exit 0
