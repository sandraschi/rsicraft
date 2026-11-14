# Regenerate docs/GUIDE.md and docs/help/index.html from the built-in help
# (crates/rsicraft-core/src/help.rs), so the CLI, MCP, studio and documents
# all say the same thing. Run after editing help.rs:
#   powershell -File scripts\build-help-docs.ps1
param(
    [string]$Exe = ""
)
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

if (-not $Exe) {
    cargo build -q -p rsicraft-mcp
    if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }
    $Exe = Join-Path $root "target\debug\rsicraft.exe"
}
$raw = & $Exe help-tabs --tab all --json
if ($LASTEXITCODE -ne 0) { throw "rsicraft help-tabs failed" }
$help = ($raw -join "`n") | ConvertFrom-Json

function Add-ListSpacing([string]$body) {
    # Markdown needs a blank line between a paragraph and a following list.
    $out = New-Object System.Collections.Generic.List[string]
    $prev = ""
    foreach ($line in ($body -split "`n")) {
        $isItem = $line -match '^(- |\d{1,2}\. )'
        $prevIsItem = $prev -match '^(- |\d{1,2}\. )'
        if ($isItem -and $prev -ne "" -and -not $prevIsItem) { $out.Add("") }
        if (-not $isItem -and $prevIsItem -and $line -ne "") { $out.Add("") }
        $out.Add($line)
        $prev = $line
    }
    return ($out -join "`n")
}

# ---------- docs/GUIDE.md ----------
$md = New-Object System.Text.StringBuilder
[void]$md.AppendLine("# rsicraft guide")
[void]$md.AppendLine("")
[void]$md.AppendLine("Generated from the built-in help by ``scripts/build-help-docs.ps1``. The same text is in the studio (Help page), on the command line (``rsicraft help-tabs``) and over MCP (``rsi_help``). Edit ``crates/rsicraft-core/src/help.rs``, not this file.")
[void]$md.AppendLine("")
[void]$md.AppendLine("## Contents")
[void]$md.AppendLine("")
foreach ($t in $help.tabs) {
    $anchor = ($t.title.ToLower() -replace '[^a-z0-9 -]', '' -replace ' ', '-')
    [void]$md.AppendLine("- [$($t.title)](#$anchor): $($t.summary -replace "`n", ' ')")
}
foreach ($t in $help.tabs) {
    [void]$md.AppendLine("")
    [void]$md.AppendLine("## $($t.title)")
    [void]$md.AppendLine("")
    [void]$md.AppendLine("*$($t.summary)*")
    foreach ($s in $t.sections) {
        [void]$md.AppendLine("")
        [void]$md.AppendLine("### $($s.heading)")
        [void]$md.AppendLine("")
        [void]$md.AppendLine((Add-ListSpacing $s.body))
        if ($s.code_example) {
            [void]$md.AppendLine("")
            [void]$md.AppendLine('```text')
            [void]$md.AppendLine($s.code_example)
            [void]$md.AppendLine('```')
        }
    }
}
$guide = ($md.ToString() -replace "`r`n", "`n")
[System.IO.File]::WriteAllText((Join-Path $root "docs\GUIDE.md"), $guide, (New-Object System.Text.UTF8Encoding($false)))

# ---------- docs/help/index.html ----------
function Esc([string]$s) { return [System.Net.WebUtility]::HtmlEncode($s) }
function BodyHtml([string]$body) {
    $html = New-Object System.Text.StringBuilder
    $inList = ""
    foreach ($para in ($body -split "`n`n")) {
        foreach ($line in ($para -split "`n")) {
            if ($line -match '^- (.*)$') {
                if ($inList -ne "ul") { if ($inList) { [void]$html.Append("</$inList>") }; [void]$html.Append("<ul>"); $inList = "ul" }
                [void]$html.Append("<li>$(Esc $Matches[1])</li>")
            } elseif ($line -match '^\d{1,2}\. (.*)$') {
                if ($inList -ne "ol") { if ($inList) { [void]$html.Append("</$inList>") }; [void]$html.Append("<ol>"); $inList = "ol" }
                [void]$html.Append("<li>$(Esc $Matches[1])</li>")
            } else {
                if ($inList) { [void]$html.Append("</$inList>"); $inList = "" }
                [void]$html.Append("<p>$(Esc $line)</p>")
            }
        }
        if ($inList) { [void]$html.Append("</$inList>"); $inList = "" }
    }
    return $html.ToString()
}

$nav = New-Object System.Text.StringBuilder
$panes = New-Object System.Text.StringBuilder
$first = $true
foreach ($t in $help.tabs) {
    $cls = if ($first) { ' class="on"' } else { '' }
    [void]$nav.Append("<button data-tab=`"$($t.id)`"$cls>$(Esc $t.title)</button>")
    $hidden = if ($first) { '' } else { ' hidden' }
    [void]$panes.Append("<section id=`"$($t.id)`" class=`"pane`"$hidden><h2>$(Esc $t.title) <span class=`"badge`">$(Esc $t.badge)</span></h2><p class=`"sum`">$(Esc $t.summary)</p>")
    foreach ($s in $t.sections) {
        [void]$panes.Append("<article><h3>$(Esc $s.heading)</h3>$(BodyHtml $s.body)")
        if ($s.code_example) { [void]$panes.Append("<pre>$(Esc $s.code_example)</pre>") }
        [void]$panes.Append("</article>")
    }
    [void]$panes.Append("</section>")
    $first = $false
}

$page = @"
<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>rsicraft help</title>
<style>
:root { --bg:#0b1020; --fg:#f1f5f9; --dim:#cbd5e1; --acc:#38bdf8; --amb:#fbbf24; --card:#111a2e; --line:#334155; }
@media (prefers-color-scheme: light) { :root { --bg:#f8fafc; --fg:#0f172a; --dim:#334155; --acc:#0369a1; --amb:#92400e; --card:#ffffff; --line:#cbd5e1; } }
* { box-sizing: border-box; }
body { margin:0; background:var(--bg); color:var(--fg); font:17px/1.6 system-ui, "Segoe UI", sans-serif; }
header { position:sticky; top:0; background:var(--bg); border-bottom:1px solid var(--line); padding:12px 20px; z-index:1; }
h1 { margin:0 0 8px; font-size:22px; }
nav { display:flex; flex-wrap:wrap; gap:6px; }
nav button { background:var(--card); color:var(--fg); border:1px solid var(--line); border-radius:6px; padding:6px 10px; font:inherit; font-size:15px; cursor:pointer; }
nav button.on { border-color:var(--acc); color:var(--acc); font-weight:600; }
#q { margin-top:8px; width:min(420px,100%); padding:6px 10px; font:inherit; background:var(--card); color:var(--fg); border:1px solid var(--line); border-radius:6px; }
main { max-width:900px; margin:0 auto; padding:20px; }
h2 { color:var(--acc); }
.badge { font-size:13px; color:var(--amb); border:1px solid var(--amb); border-radius:10px; padding:1px 8px; vertical-align:middle; }
.sum { color:var(--dim); font-style:italic; }
article { background:var(--card); border:1px solid var(--line); border-radius:8px; padding:4px 18px 12px; margin:14px 0; }
h3 { color:var(--amb); margin:12px 0 6px; }
pre { background:var(--bg); border:1px solid var(--line); border-radius:6px; padding:10px; overflow-x:auto; font:14px/1.45 Consolas, monospace; }
.miss { display:none; }
</style>
</head>
<body>
<header>
<h1>rsicraft help</h1>
<nav>$($nav.ToString())</nav>
<input id="q" type="search" placeholder="Search all tabs (e.g. timeout, docker, Ollama)">
</header>
<main>$($panes.ToString())</main>
<script>
const buttons = document.querySelectorAll('nav button');
const panes = document.querySelectorAll('.pane');
function show(id) {
  buttons.forEach(b => b.classList.toggle('on', b.dataset.tab === id));
  panes.forEach(p => p.hidden = p.id !== id);
}
buttons.forEach(b => b.addEventListener('click', () => { document.getElementById('q').value = ''; filter(''); show(b.dataset.tab); }));
function filter(q) {
  q = q.trim().toLowerCase();
  document.querySelectorAll('article').forEach(a => a.classList.toggle('miss', q !== '' && !a.textContent.toLowerCase().includes(q)));
  if (q === '') { const on = document.querySelector('nav button.on'); show(on ? on.dataset.tab : panes[0].id); return; }
  panes.forEach(p => p.hidden = !p.querySelector('article:not(.miss)'));
}
document.getElementById('q').addEventListener('input', e => filter(e.target.value));
</script>
</body>
</html>
"@
$page = $page -replace "`r`n", "`n"
[System.IO.File]::WriteAllText((Join-Path $root "docs\help\index.html"), $page, (New-Object System.Text.UTF8Encoding($false)))
Write-Output "wrote docs/GUIDE.md and docs/help/index.html from $($help.tabs.Count) tabs"
