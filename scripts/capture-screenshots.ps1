# Capture README screenshots of the studio, one per page, into docs/screenshots.
# Needs an interactive desktop session (the window must actually be drawn).
#
#   powershell -File scripts\capture-screenshots.ps1 -Database <file> -Task <task-id>
#
# Use a database with a real search in it, so the pages show real data.
param(
    [Parameter(Mandatory = $true)][string]$Database,
    [string]$Task = "primes",
    [string]$Exe = "",
    [string]$LlmUrl = "http://127.0.0.1:11434",
    [string]$Model = "",
    [int]$SettleMs = 2500
)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class RsiWindowCapture {
    [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
    public delegate bool EnumWindowsProc(IntPtr hWnd, IntPtr lParam);
    [DllImport("user32.dll")] public static extern bool EnumWindows(EnumWindowsProc f, IntPtr p);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
    [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr hdc, uint flags);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
    [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
    public static IntPtr FindMainWindow(uint pid) {
        IntPtr found = IntPtr.Zero; int best = 0;
        EnumWindows((h, p) => {
            uint id; GetWindowThreadProcessId(h, out id);
            if (id == pid && IsWindowVisible(h)) {
                RECT r;
                if (GetWindowRect(h, out r)) {
                    int w = r.Right - r.Left, ht = r.Bottom - r.Top;
                    if (w > 400 && ht > 300 && w * ht > best) { best = w * ht; found = h; }
                }
            }
            return true;
        }, IntPtr.Zero);
        return found;
    }
}
"@

# Without this, window coordinates are scaled on high-DPI screens and the capture is clipped.
[RsiWindowCapture]::SetProcessDPIAware() | Out-Null

$root = Split-Path -Parent $PSScriptRoot
if (-not $Exe) { $Exe = Join-Path $root 'target\release\rsicraft-app.exe' }
if (-not (Test-Path -LiteralPath $Exe)) { throw "studio exe not found: $Exe (cargo build --release -p rsicraft-app)" }
if (-not (Test-Path -LiteralPath $Database)) { throw "database not found: $Database" }
$outDir = Join-Path $root 'docs\screenshots'
New-Item -ItemType Directory -Force -Path $outDir | Out-Null

$shots = @(
    @{ Name = 'studio';  Page = 'studio'; Extra = @() },
    @{ Name = 'lineage'; Page = 'lineage'; Extra = @() },
    @{ Name = 'cards';   Page = 'cards'; Extra = @() },
    @{ Name = 'help';    Page = 'help'; Extra = @() }
)

$env:RSICRAFT_DB = (Resolve-Path -LiteralPath $Database).Path
$env:RSICRAFT_LLM_URL = $LlmUrl
if ($Model) { $env:RSICRAFT_MODEL = $Model }
foreach ($s in $shots) {
    $png = Join-Path $outDir "$($s.Name).png"
    $proc = Start-Process -FilePath $Exe -ArgumentList (@('--task', $Task, '--page', $s.Page, '--provider', 'llama', '--sandbox', 'local') + $s.Extra) -PassThru
    try {
        $hwnd = [IntPtr]::Zero
        for ($i = 0; $i -lt 40; $i++) {
            Start-Sleep -Milliseconds 250
            $hwnd = [RsiWindowCapture]::FindMainWindow([uint32]$proc.Id)
            if ($hwnd -ne [IntPtr]::Zero) { break }
        }
        if ($hwnd -eq [IntPtr]::Zero) { throw "no visible studio window for $($s.Name)" }
        [RsiWindowCapture]::SetForegroundWindow($hwnd) | Out-Null
        Start-Sleep -Milliseconds $SettleMs
        $r = New-Object RsiWindowCapture+RECT
        [RsiWindowCapture]::GetWindowRect($hwnd, [ref]$r) | Out-Null
        $w = [Math]::Max(1, $r.Right - $r.Left); $h = [Math]::Max(1, $r.Bottom - $r.Top)
        $bmp = New-Object System.Drawing.Bitmap($w, $h)
        $g = [System.Drawing.Graphics]::FromImage($bmp)
        $hdc = $g.GetHdc()
        $ok = [RsiWindowCapture]::PrintWindow($hwnd, $hdc, 2)
        $g.ReleaseHdc($hdc)
        $bmp.Save($png, [System.Drawing.Imaging.ImageFormat]::Png)
        $g.Dispose(); $bmp.Dispose()
        Write-Output "captured $($s.Name).png ($w x $h, ok=$ok)"
    } finally {
        if (-not $proc.HasExited) { Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue; Start-Sleep -Milliseconds 300 }
    }
}
