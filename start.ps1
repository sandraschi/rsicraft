# RSICraft startup script: builds and launches the native eframe/egui 0.37 GPU Desktop Studio.
# Pass -Mcp to run the stdio MCP 2024-11-05 server instead.
# Pass -Cli to run the CLI status check instead.
# Pass -Release to run optimized binary.
param(
    [switch]$Mcp,
    [switch]$Cli,
    [switch]$Release
)

$ErrorActionPreference = "Stop"

$cargoBin = Join-Path $env:USERPROFILE ".cargo\bin"
if (Test-Path -LiteralPath $cargoBin) {
    $env:Path = "$cargoBin;$env:Path"
}

if ($null -eq (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Host "[ERROR] cargo not found. Please install Rust from https://rustup.rs and ensure ~/.cargo/bin is in PATH." -ForegroundColor Red
    exit 1
}

$repoRoot = $PSScriptRoot
Push-Location -LiteralPath $repoRoot
try {
    $relFlag = if ($Release) { "--release" } else { "" }

    if ($Mcp) {
        Write-Host "Starting RSICraft stdio MCP 2024-11-05 server (Ctrl+C to stop)..." -ForegroundColor Cyan
        if ($Release) {
            & cargo run --release -p rsicraft-mcp -- mcp
        } else {
            & cargo run -p rsicraft-mcp -- mcp
        }
        return
    }

    if ($Cli) {
        Write-Host "Running RSICraft CLI status check..." -ForegroundColor Cyan
        if ($Release) {
            & cargo run --release -p rsicraft-mcp -- status
        } else {
            & cargo run -p rsicraft-mcp -- status
        }
        return
    }

    Write-Host "Launching RSICraft Sovereign GPU Studio (eframe / egui 0.37)..." -ForegroundColor Cyan
    Write-Host "  Features: 3D RTX 4090 Viewport, 3D Fitness Manifold, Bezier DAG & 9-Tab Help" -ForegroundColor Gray
    if ($Release) {
        & cargo run --release -p rsicraft-app
    } else {
        & cargo run -p rsicraft-app
    }
} finally {
    Pop-Location
}
