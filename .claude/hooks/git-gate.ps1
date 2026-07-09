# PreToolUse hook for Claude Code Bash tool calls (Windows / PowerShell).
# BLOCKS `git commit` / `git push` if the testing methodology's static
# layers fail (formatter / linter / unit + contract tests).
#
# Contract per methodology-toolkit/METHODOLOGY.md § 8:
#   stdin: JSON {"tool_input":{"command":"git commit -m foo"}}
#   exit 0 -> allow
#   exit 2 -> block with stderr shown to operator
#
# IMPORTANT: After editing this script OR .claude/settings.json the
# Claude Code shell must be RESTARTED.

$ErrorActionPreference = "Stop"

# === STACK BINDINGS (Rust, boosty_api) ==================================

$ToolchainBin = Join-Path $env:USERPROFILE ".cargo\bin"

$FmtCheckGate = @{
    Name = "formatter --check"
    Exe  = Join-Path $ToolchainBin "cargo.exe"
    Args = @("fmt", "--all", "--", "--check")
}
$LintGate = @{
    Name = "linter --strict"
    Exe  = Join-Path $ToolchainBin "cargo.exe"
    Args = @("clippy", "--all-targets", "--", "-D", "warnings")
}
$TypeCheckGate = $null    # built into clippy/rustc

$UnitTestGate = @{
    Name = "unit tests"
    Exe  = Join-Path $ToolchainBin "cargo.exe"
    Args = @("test", "--lib", "--quiet")
}
# All offline integration tests; tests/live_api.rs is #[ignore]d and stays offline.
$ContractTestGate = @{
    Name = "contract tests"
    Exe  = Join-Path $ToolchainBin "cargo.exe"
    Args = @("test", "--tests", "--quiet")
}

# === HOOK LOGIC — usually no edits below ================================

# --- Read stdin JSON ----------------------------------------------------
$raw = [Console]::In.ReadToEnd()
if (-not $raw) { exit 0 }

try {
    $payload = $raw | ConvertFrom-Json
} catch {
    [Console]::Error.WriteLine("[git-gate] WARN: failed to parse stdin JSON; allowing.")
    exit 0
}

$cmd = ""
if ($payload.tool_input -and $payload.tool_input.command) {
    $cmd = [string]$payload.tool_input.command
}

# --- Fast exit if not git commit/push ----------------------------------
$isCommit = $cmd -match '\bgit\s+commit\b'
$isPush   = $cmd -match '\bgit\s+push\b'
if (-not ($isCommit -or $isPush)) {
    exit 0
}

# --- --no-verify bypass ------------------------------------------------
if ($cmd -match '--no-verify') {
    [Console]::Error.WriteLine("[git-gate] WARN: --no-verify bypasses methodology gates.")
    exit 0
}

# --- Toolchain check ---------------------------------------------------
if (-not (Test-Path $ToolchainBin)) {
    [Console]::Error.WriteLine("[git-gate] WARN: toolchain dir $ToolchainBin missing -- gate disabled.")
    exit 0
}

# --- Gate runner (uses Start-Process to avoid PS 5.1 2>&1 trap) --------
function Invoke-Gate($gate) {
    if (-not $gate) { return }      # skip empty bindings

    [Console]::Error.WriteLine("[git-gate] $($gate.Name) ...")
    $tmpOut = [System.IO.Path]::GetTempFileName()
    $tmpErr = [System.IO.Path]::GetTempFileName()
    try {
        $p = Start-Process -FilePath $gate.Exe -ArgumentList $gate.Args -NoNewWindow `
            -RedirectStandardOutput $tmpOut -RedirectStandardError $tmpErr `
            -Wait -PassThru
        if ($p.ExitCode -ne 0) {
            [Console]::Error.WriteLine("")
            [Console]::Error.WriteLine("[git-gate] BLOCK: $($gate.Name) failed (exit $($p.ExitCode)).")
            [Console]::Error.WriteLine("[git-gate] Last 30 lines of output:")
            $combined = @()
            if (Test-Path $tmpOut) { $combined += Get-Content $tmpOut }
            if (Test-Path $tmpErr) { $combined += Get-Content $tmpErr }
            ($combined | Select-Object -Last 30) | ForEach-Object {
                [Console]::Error.WriteLine("  $_")
            }
            [Console]::Error.WriteLine("")
            [Console]::Error.WriteLine("[git-gate] Fix and re-commit. --no-verify bypasses (NOT recommended).")
            exit 2
        }
    } finally {
        Remove-Item -Force -ErrorAction SilentlyContinue $tmpOut, $tmpErr
    }
}

# --- Always: format + lint + (optional) type-check ---------------------
Invoke-Gate $FmtCheckGate
Invoke-Gate $LintGate
Invoke-Gate $TypeCheckGate

# --- Push-only: full test suite ----------------------------------------
if ($isPush) {
    Invoke-Gate $UnitTestGate
    Invoke-Gate $ContractTestGate
}

[Console]::Error.WriteLine("[git-gate] All gating layers green. Proceeding with: $cmd")
exit 0
