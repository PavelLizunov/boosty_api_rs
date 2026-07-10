# AGENTS.md

Agent onboarding for `boosty_api` (Rust). Read this first, then read
`docs/IMPLEMENTATION.md` for the full, self-contained design + bridge spec.
`CLAUDE.md` is the Claude-Code variant of the same rules — you don't need it,
but it is the source of truth if the two ever disagree.

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
- **Next phase (Phase 2): the Boosty↔vpnctl bridge**, built in the SEPARATE
  `vpnctl` repo, consuming this crate. This crate is feature-complete for it.
- In THIS repo there is no pending code task unless the live API drifts (see
  "Model / API changes"). To build the bridge, switch to the `vpnctl` repo and
  follow `docs/IMPLEMENTATION.md` Part II — and get the § 12 open questions
  answered by the user before its Phase C.

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

The blocking pre-commit gate is a **Claude Code hook**
(`.claude/hooks/git-gate.ps1`, wired via `.claude/settings.json`). It **does
NOT run under Codex.** Enforcement is therefore on you:

- **Run `scripts/ci.ps1` and confirm it is green before every commit.** Same
  bar as the hook: fmt clean, clippy `-D warnings`, unit + contract tests
  pass, `cargo audit` clean.
- Model/API-contract changes additionally require the live canary (below).
- There is nothing to `--no-verify` around — just never commit red.
- **No pushes or releases without the user's explicit go.**

## Hard invariants (each one caused a real bug or a security issue)

- All HTTP goes through `ApiClient::send_authorized` (auth header + single
  retry-on-401). Never call `self.client.get(...)` from endpoint modules.
- Every response passes `handle_response` (status check) before `parse_json`.
  No exceptions — DELETE included.
- Every string interpolated into a URL path or query goes through
  `api_client::encode_segment`. Numeric params are exempt; `serde_urlencoded`
  output is already encoded — do not double-encode it.
- Credentials are never printed: `AuthState` has a manual redacting `Debug`,
  and a failed refresh keeps only the first 200 chars of the response body.
  Keep both. Tokens travel only in the `Authorization` header.
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

## Model / API changes

Offline fixtures lie — a `price: i32` passed fixtures but broke on live
`0.98`. Any change to models or request construction needs an offline mockito
test AND a run of the live canary. `tests/audit_probes.rs` is the regression
suite: 8 probes, each documenting a formerly-real bug — keep them all green,
and add one when you fix anything security-shaped.

## Secrets & safety

- Never print tokens to chat, logs, or panic messages.
- `.secrets/boosty.json` is gitignored and dev/test only — never copy it to a
  server (see ADR-001 in `docs/IMPLEMENTATION.md`).
- Live tests use small limits against public, anonymous data. Be a polite API
  citizen.

## Environment quirks

- PS 5.1 `Get-Content` / `Set-Content` mangle UTF-8 (mojibake) — use an editor
  or proper file APIs for source files, not those cmdlets.
- This repo's CI, hook, and helper scripts are PowerShell; the host is
  Windows 11.
