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
| 1 — Messages + subscribers | done | dialogs/messages + own-blog subscribers, verified live |
| 2 — New features | proposed | per user's upcoming tasks |

Phase-2 north star — the Boosty↔vpnctl bridge — has a full self-contained
handoff spec in `docs/IMPLEMENTATION.md` (crate as-built + bridge design,
written for any agent incl. non-Claude).

Live-verified endpoints (2026-07): dialogs `GET /v1/dialog/`,
messages `GET /v1/dialog/{id}/message/` (offset = last msg id, stop on
`isLast`), subscribers `GET /v1/blog/{blog}/subscribers` (NO trailing
slash — the slashed path 404s; offset-based, `sort_by`/`order` optional).
Own blog url comes from `GET /v1/user/current` → `blogUrl` (not modeled;
the live test reads it raw).

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
- Any string interpolated into a URL path or query goes through
  `api_client::encode_segment` (blog urls come back from API responses;
  unencoded `/`, `?`, `#` reroute the request). Numeric params are
  exempt; serde_urlencoded output is already encoded — don't re-encode.
- Long-running consumers (the vpnctl bridge) MUST build the reqwest
  `Client` with `connect_timeout` + `timeout`: token refresh holds the
  auth mutex across the network call, so one hung connection stalls
  every request on the client.

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
| 4 | dependency audit | `cargo audit` (in ci.ps1; git-gate runs it on push only — fetches the RustSec DB) |

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
- Authenticated live tests read the gitignored `.secrets/boosty.json`
  (`access_token`, `refresh_token`, `device_id`). To fill it: log in at
  boosty.to → F12 → Application → Local Storage → https://boosty.to →
  key `auth` holds `accessToken`/`refreshToken`, key `_clentId` is the
  device id. Fill the file in an editor — NEVER paste tokens into chat;
  tests never print them. `live_auth_refresh_flow` additionally needs
  `BOOSTY_TEST_REFRESH=1` because it rotates the refresh token (it
  writes the rotated value back to the file; the browser session that
  produced it may need a re-login).

## 9. ADRs

### ADR-001 — vpnctl bridge secret storage (2026-07-10)

The bridge gets Boosty credentials (access/refresh token, device_id)
injected at runtime from the vpnctl side — its existing secret
mechanism, or env vars if it has none. This repo's `.secrets/*.json`
pattern stays dev/test-only and is never copied to a server. Boosty
rotates the refresh token on every successful refresh, so the bridge
MUST persist the rotated value (`ApiClient::refresh_token()`) back to
its store after each refresh — otherwise the credentials go stale.
Tokens are never logged (redacting `Debug`, truncated refresh-error
body — probes 6/7).
