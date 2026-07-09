# Project memory — boosty_api (fork)

Auto-injected into every Claude Code session for this project.
Structure follows methodology-toolkit/METHODOLOGY.md § 12.

## 1. Strategic context

`boosty_api` is an async Rust client for the Boosty platform API
(posts, comments, targets, subscription levels, showcase, bundles).
This repo is PavelLizunov's fork of `ath31st/boosty_api_rs`, extended
for personal use. North star: a reliable, honest client whose models
match the *live* API — every claimed behavior is backed by a test.

## 2. Roadmap

| Phase | Status | Notes |
|---|---|---|
| 0 — Audit + hardening | done | deps, retry-on-401, token safety, live contract tests |
| 1 — New features | proposed | per user's upcoming tasks |

## 3. Workflow rules (BLOCKING)

```
1. review          — inline self-review of the full diff by the main
                     model (user directive: NO subagents, ever)
2. local ci        — scripts/ci.ps1  (fmt --check, clippy -D warnings,
                     unit + offline integration tests)
3. live canary     — cargo test --test live_api -- --ignored --nocapture
                     (model/API-contract changes only; hits api.boosty.to)
4. git commit/push — auto-gated by .claude/hooks/git-gate.ps1
```

No pushes or releases without the user's explicit go.

## 4. Architectural invariants

- All HTTP goes through `ApiClient::send_authorized` (auth header +
  single retry-on-401 via forced refresh). Never call
  `self.client.get(...)` directly from endpoint modules.
- Every endpoint response passes `handle_response` (status check)
  before `parse_json`. No exceptions — DELETE included.
- Credentials are never printed: `AuthState` has a manual redacting
  `Debug`. Keep it that way for any new state that holds secrets.
- Price-like fields are `f64` — the live API returns fractional values
  (e.g. `Post.price = 0.98`). Do not "clean up" to integers.
- Comments pagination flags are CHRONOLOGICAL, not directional
  (verified live 2026-07): the terminal page in any order is exactly
  `isFirst && isLast`. Do not "simplify" the loop condition in
  `get_all_comments`.
- `tokio` in `[dependencies]` stays `features = ["sync"]` only; the
  runtime/macros live in `[dev-dependencies]`.
- reqwest 0.13: `.form()` requires the `form` feature.
- New model types must be re-exported from `src/model.rs` — no
  pub-but-unnameable types.

## 5. Operator-action policy

- Never print tokens/credentials to chat, logs, or panic messages.
- Never spawn subagents; all work is done inline by the main model.
- Be a polite API citizen: live tests use small limits against public
  anonymous data only.
- No `--no-verify` commits except a true ≤5-line single-surface hotfix.

## 6. Methodology layers (non-UI form)

| # | Layer | Command |
|---|---|---|
| 1 | static checks | `cargo fmt --all -- --check` && `cargo clippy --all-targets -- -D warnings` |
| 2 | unit tests | `cargo test --lib --quiet` |
| 3 | contract tests | `cargo test --tests --quiet` (offline; mockito) |
| 3b | live canary | `cargo test --test live_api -- --ignored --nocapture` |

Layer 5/6 (install/visual): N/A — this is a library.

## 7. Lessons learned

- 2026-07-09 — `Post.price: i32` broke on live data (`0.98`); offline
  fixtures never caught it → live canary test added
  (`tests/live_api.rs`), price fields switched to `f64`.
- 2026-07-09 — upstream's odd `is_last && is_first` loop condition
  looked like a bug but is CORRECT per live flag semantics → verify
  against the live API before "fixing" upstream logic.
- 2026-07-09 — `derive(Debug)` on auth state leaked bearer tokens into
  any `{:?}` log → manual redacting Debug + regression probe.

## 8. Environment quirks

- cargo is NOT on PowerShell PATH: prepend
  `$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"` per session.
- PS 5.1 `Get-Content`/`Set-Content` mangles UTF-8 (mojibake) — use
  the Edit tool or `[System.IO.File]` with explicit UTF-8.
- `tests/audit_probes.rs` is the regression suite from the 2026-07
  audit; each test documents a formerly-real bug. Keep them green.
