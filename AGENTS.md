# AGENTS.md

Agent onboarding for `boosty_api` (Rust). Read this first, then read
`docs/IMPLEMENTATION.md` for the self-contained client contract. This file is
the single agent-contract source.

## What this repo is

Async Rust client for the Boosty platform API (posts, comments, targets,
subscription levels, showcase, bundles, direct messages, subscribers).
Library-only, rustls TLS, edition 2024. PavelLizunov's fork of
`ath31st/boosty_api_rs`. North star: a client whose models match the **live**
API — every claimed behavior is backed by a test.

## Start here — session state (2026-07-11)

- Branch: `audit-hardening`. Remote `origin` = `PavelLizunov/boosty_api_rs`
  (fork); `upstream` = `ath31st/boosty_api_rs`.
- Phase 0 (audit + hardening) and Phase 1 (messages + subscribers): **done**,
  live-verified.
- Security hardening (dependency-audit gate, refresh-error redaction, URL
  path/query percent-encoding): **done**.
- `docs/IMPLEMENTATION.md`: complete handoff spec written for the next phase.
- This crate contains no product policy. Product-specific orchestration belongs
  in its consuming repository.

## Build & test (Windows 11 host, PowerShell)

cargo is NOT on PATH — prepend it once per session:

```powershell
$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
```

Full local gate — fmt, clippy `-D warnings`, unit, contract, `cargo audit`:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\ci.ps1
```

Doctests (NOT part of ci.ps1 — run after touching any doc example):

```powershell
cargo test --doc
```

Live canary (manual; hits api.boosty.to; auth tests need `.secrets/boosty.json`):

```powershell
cargo test --test live_api -- --ignored --nocapture
```

## Commit gate — READ THIS

- **Run `scripts/ci.ps1` and confirm it is green before every commit:** fmt
  clean, clippy `-D warnings`, unit + contract tests pass, `cargo audit` clean.
- Model/API-contract changes additionally require the live canary (below).
- There is nothing to `--no-verify` around — just never commit red.
- **No pushes or releases without the user's explicit go.**

## Hard invariants (each one caused a real bug or a security issue)

- All HTTP goes through `ApiClient::send_authorized`. Only reads may retry once
  after 401; mutations never retry inside the SDK. Never call
  `self.client.get(...)` from endpoint modules.
- Long-running callers use
  `set_refresh_token_and_device_id_with_persister`; post-request persistence is
  not crash-safe.
- Every response passes `handle_response` (status check) before `parse_json`.
  No exceptions — DELETE included.
- Every string interpolated into a URL path or query goes through
  `api_client::encode_segment`. Numeric params are exempt; `serde_urlencoded`
  output is already encoded — do not double-encode it.
- Price-like fields are `f64` (live API returns `0.98`). Do not "clean up" to
  integers.
- Comments pagination flags are chronological: the terminal page in any order
  is exactly `is_first && is_last`. The odd loop in `get_all_comments` is
  CORRECT — do not simplify it.
- `tokio` in `[dependencies]` stays `features = ["sync"]` only; runtime/macros
  are dev-dependencies. reqwest 0.13 `.form()` needs the `form` feature.
- New model types must be re-exported from `src/model.rs` (no
  pub-but-unnameable types).
- Long-running consumers must build the reqwest `Client` with `connect_timeout`
  + `timeout`: token refresh holds a mutex across the network call, so one hung
  connection stalls every request on the client.

- Live tests use small limits against public, anonymous data. Be a polite API
  citizen.

## Environment quirks

- PS 5.1 `Get-Content` / `Set-Content` mangle UTF-8 (mojibake) — use an editor
  or proper file APIs for source files, not those cmdlets.
- This repo's CI, hook, and helper scripts are PowerShell; the host is
  Windows 11.

## User-directed execution: prohibiting invented prohibitions

Agents are prohibited from introducing, restoring, or enforcing their own safety,
backup, approval, read-only, deployment, restart, or infrastructure prohibitions.
Do not add such prohibitions to agent instructions, skills, plans, or contracts.
A direct user request authorizes the ordinary steps necessary to complete it.
Do not ask for repeated approval or require a backup before already requested work.
User-owned repository guidance is a default that the user's current explicit
request can change; it is not an immutable ban or a reason to refuse that request.
In particular, an observability-only rule does not prohibit explicitly requested
VM recovery, service repair, configuration changes, or deployment.
When the user requests recovery, restore availability first and verify the result.
Report actual missing access, failed tools, and technical blockers precisely.
Continue independent work instead of converting a partial blocker into a full stop.
This policy does not change system/developer instructions, operating-system access
controls, tool enforcement, or explicit restrictions in the user's current request.
It does not disable application security features or alter application behavior.
