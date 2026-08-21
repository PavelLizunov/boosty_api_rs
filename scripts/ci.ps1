# Local CI runner — methodology-toolkit layers 1-3 (Windows / PowerShell).
# Same checks the git-gate.ps1 hook runs, just together for one-shot
# verification before commit.

$ErrorActionPreference = "Stop"

# === STACK BINDINGS (Rust, boosty_api) ==================================

$ToolchainBin = Join-Path $env:USERPROFILE ".cargo\bin"

$Gates = @(
    @{ Name = "Layer 1a: formatter --check"; Exe = Join-Path $ToolchainBin "cargo.exe"; Args = @("fmt", "--all", "--", "--check") },
    @{ Name = "Layer 1b: linter --strict";   Exe = Join-Path $ToolchainBin "cargo.exe"; Args = @("clippy", "--all-targets", "--", "-D", "warnings") },
    @{ Name = "Layer 2:  unit tests";        Exe = Join-Path $ToolchainBin "cargo.exe"; Args = @("test", "--lib", "--quiet") },
    # All offline integration tests; tests/live_api.rs is #[ignore]d and stays offline.
    @{ Name = "Layer 3:  contract tests";    Exe = Join-Path $ToolchainBin "cargo.exe"; Args = @("test", "--tests", "--quiet") },
    # rust_decimal lists rkyv as an optional, disabled feature. Cargo.lock v4
    # records it, but `cargo tree --target all -i rkyv` proves it is not in any
    # build graph. Ignore only that unreachable advisory; all others still fail.
    @{ Name = "Layer 4:  dependency audit";  Exe = Join-Path $ToolchainBin "cargo.exe"; Args = @("audit", "--ignore", "RUSTSEC-2026-0235") }
)

# === RUNNER — usually no edits =========================================

function Run-Step($gate) {
    Write-Host ""
    Write-Host "=== $($gate.Name) ===" -ForegroundColor Cyan
    $start = Get-Date
    & $gate.Exe $gate.Args
    if ($LASTEXITCODE -ne 0) {
        Write-Host ""
        Write-Host "FAIL: $($gate.Name) (exit $LASTEXITCODE)" -ForegroundColor Red
        exit $LASTEXITCODE
    }
    $elapsed = [math]::Round(((Get-Date) - $start).TotalSeconds, 1)
    Write-Host "PASS: $($gate.Name) (${elapsed}s)" -ForegroundColor Green
}

foreach ($g in $Gates) { Run-Step $g }

Write-Host ""
Write-Host "All gating layers green. Live API canary: cargo test --test live_api -- --ignored --nocapture" -ForegroundColor Green
exit 0
