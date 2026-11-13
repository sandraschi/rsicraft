# Analyze a bench-models.ps1 output directory.
#
# For every run database it reports the first working candidate, the best
# DRAFT, the final best, which operator produced the best, and the search
# amplification (final best / first working candidate).
#
# Amplification is the number that decides whether a task pack is worth
# training on: if search does not clearly beat the model's first working
# answer, there is nothing for a next model generation to learn from.
#
# Example:
#   powershell -File scripts\bench-analyze.ps1 -Dir $env:LOCALAPPDATA\rsicraft\bench\modelswap-v2
param(
    [Parameter(Mandatory = $true)][string]$Dir,
    [string]$Exe = ''
)

$ErrorActionPreference = 'Continue'
if (-not $Exe) { $Exe = Join-Path $Dir 'rsicraft.exe' }
if (-not (Test-Path -LiteralPath $Exe)) {
    Write-Host "[ERROR] $Exe not found" -ForegroundColor Red
    exit 1
}
$inv = [cultureinfo]::InvariantCulture

$rows = @()
Get-ChildItem -LiteralPath $Dir -Directory | ForEach-Object {
    $model = $_.Name
    Get-ChildItem -LiteralPath $_.FullName -Filter '*.db' | ForEach-Object {
        $task = $_.BaseName -replace '-t\d+$', ''
        $bundle = $null
        try { $bundle = ((& $Exe --db $_.FullName --json export --task-id $task 2>$null) -join "`n") | ConvertFrom-Json } catch { $bundle = $null }
        if ($null -eq $bundle -or $null -eq $bundle.cards) { return }
        $cards = @($bundle.cards | Sort-Object created_at)
        $ok = @($cards | Where-Object { $_.execution_status -eq 'SUCCESS' })
        if ($ok.Count -eq 0) {
            $rows += [pscustomobject]@{ model = $model; task = $task; run = $_.BaseName; cards = $cards.Count; ok = 0; first = 0; best_draft = 0; best = 0; best_op = '-'; amplification = 0 }
            return
        }
        $first = $ok[0]
        $best = $ok | Sort-Object primary_score -Descending | Select-Object -First 1
        $drafts = @($ok | Where-Object { $_.operator_used -eq 'DRAFT' })
        $bestDraft = if ($drafts.Count) { ($drafts | Measure-Object primary_score -Maximum).Maximum } else { 0 }
        $rows += [pscustomobject]@{
            model         = $model
            task          = $task
            run           = $_.BaseName
            cards         = $cards.Count
            ok            = $ok.Count
            first         = [math]::Round($first.primary_score, 2)
            best_draft    = [math]::Round($bestDraft, 2)
            best          = [math]::Round($best.primary_score, 2)
            best_op       = $best.operator_used
            amplification = [math]::Round($best.primary_score / [math]::Max($first.primary_score, 0.01), 2)
        }
    }
}

if ($rows.Count -eq 0) {
    Write-Host 'no run databases found'
    exit 0
}

$rows | Sort-Object task, model, run | Format-Table -AutoSize | Out-String -Width 220 | Write-Host

Write-Host 'Per model and task (mean over runs with a working candidate):'
$rows | Where-Object { $_.ok -gt 0 } | Group-Object model, task | Sort-Object Name | ForEach-Object {
    $g = $_.Group
    '{0,-28} {1,-14} runs={2} mean_best={3,10} mean_amplification={4,6}' -f $g[0].model, $g[0].task, $g.Count,
        (($g | Measure-Object best -Average).Average).ToString('F2', $inv),
        (($g | Measure-Object amplification -Average).Average).ToString('F2', $inv)
} | Write-Host

$amps = @($rows | Where-Object { $_.ok -gt 0 } | ForEach-Object { $_.amplification } | Sort-Object)
$median = $amps[[int][math]::Floor(($amps.Count - 1) / 2)]
$byDraft = @($rows | Where-Object { $_.best_op -eq 'DRAFT' }).Count
Write-Host ('Median amplification: {0}   Runs whose best card is a DRAFT: {1} of {2}' -f $median.ToString('F2', $inv), $byDraft, $rows.Count)

$rows | Export-Csv -LiteralPath (Join-Path $Dir 'analysis.csv') -NoTypeInformation -Encoding ASCII
