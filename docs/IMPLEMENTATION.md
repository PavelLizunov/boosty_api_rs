# Boosty↔VPN bridge — implementation handoff

Self-contained spec for any coding agent (Claude Code, Codex, or a human).
It documents (I) the `boosty_api` crate as actually built and verified, and
(II) the bridge that consumes it inside the **vpnctl** admin project.
No other context is required to start; everything stated here about the
crate is backed by a test or was verified against the live API in 2026-07.

Repos:

| Repo | Role | Where |
|---|---|---|
| `boosty_api_rs` (this one) | async Rust client for the Boosty platform API | `https://github.com/PavelLizunov/boosty_api_rs`, branch `audit-hardening` (fork of `ath31st/boosty_api_rs`) |
| `vpnctl` | the user's VPN admin panel; the bridge lives THERE | separate repo on the user's machine |

North star: **map active Boosty subscribers to VPN access in vpnctl,
automatically and safely.** The crate side is finished; the bridge is not
started.

---

## Part I — the `boosty_api` crate, as built

### 1. Facts

- Crate `boosty_api` v0.28.0, Rust edition 2024, library-only.
- TLS: **rustls** (`reqwest 0.13`, `default-features = false`) — no
  openssl-sys/native-tls anywhere in the tree, so the crate embeds into
  workspaces that forbid them (vpnctl does).
- `tokio` in `[dependencies]` has `features = ["sync"]` ONLY (the lib just
  needs `tokio::sync::Mutex`); runtime/macros are dev-dependencies. Do not
  "fix" this.
- Error handling: `thiserror` enums `ApiError` / `AuthError`
  (`src/error.rs`); every fallible fn returns `ResultApi<T>` or
  `ResultAuth<T>`.
- Module map: `src/api_client.rs` (client core + one file per endpoint
  group under `src/api_client/`), `src/auth_provider.rs` (token
  lifecycle), `src/model.rs` + `src/model/` (serde models, all re-exported
  from `model.rs`), `src/helper.rs` (`handle_response`/`parse_json`),
  `src/media_content.rs` + `src/traits.rs` (content extraction).

### 2. Authentication architecture

Two mutually exclusive modes on one `ApiClient` (setting one clears the
other):

1. **Static bearer**: `set_bearer_token(access_token)` — used as-is, no
   refresh, no retry-on-401 recovery.
2. **Refresh flow**: `set_refresh_token_and_device_id(refresh, device_id)`
   — the client obtains/refreshes the access token itself:
   - proactive refresh when the cached token has ≤30 s left (or none yet);
   - on `401` from any endpoint: **exactly one** forced refresh + retry
     (regression probe 3 pins "2 attempts total"); multipart (streaming)
     requests are the exception — they are not retried;
   - refresh = `POST {base}/oauth/token/` with form fields `device_id`,
     `device_os=web`, `grant_type=refresh_token`, `refresh_token`.

**Rotation (critical for the bridge):** Boosty rotates the refresh token on
EVERY successful refresh; the old one is dead. The current value is exposed
via `ApiClient::refresh_token() -> Option<String>`. Whoever runs the client
long-term MUST persist that value after activity, or the credentials go
stale on restart. A given refresh-token lineage has a single owner: once
the bridge starts refreshing with it, the browser session that produced it
will eventually be logged out — that is expected.

**Locking:** token state sits behind a `tokio::sync::Mutex`; the refresh
HTTP call happens while holding it (intentional — prevents refresh
stampedes). Consequence: **build the `reqwest::Client` with
`connect_timeout` + `timeout`, always.** With no timeout, one hung
connection stalls every request on the client forever.

**Secret hygiene (do not regress):**
- `AuthState` has a manual redacting `Debug` (`Some(***)`); `{:?}` on the
  client never prints tokens (probe 6).
- A failed refresh keeps only the first **200 chars** of the server
  response body in `AuthError::HttpStatus` — OAuth servers may echo
  request params (incl. the refresh token) into error bodies, and that
  Display string ends up in caller logs (probe 7).
- Tokens travel only in the `Authorization` header, never in URLs.

### 3. Request pipeline (hard invariants)

Every endpoint method follows this exact shape — new endpoints must too:

```text
build path (encode_segment on EVERY string interpolated into path/query)
  → self.get_request/post_request/put_request/delete_request/post_multipart
      (all funnel into send_authorized: default headers + auth header +
       single 401-retry)
  → self.handle_response(path, response)   // status check, NO exceptions
  → self.parse_json(response)              // serde_path_to_error diagnostics
```

- Never call `self.client.get(...)` directly from endpoint modules.
- `encode_segment` (`src/api_client.rs`) percent-encodes RFC 3986
  non-unreserved bytes. String values can come back from API responses
  (e.g. blog urls) — unencoded `/`, `?`, `#` would reroute the request
  (probe 8, live-verified). Numeric params are exempt.
  `serde_urlencoded` output is already encoded — do not re-encode it.
- Price-like fields are `f64` — the live API returns fractional values
  (`price = 0.98` seen live). Do not "clean up" to integers.

### 4. Live-verified endpoint quirks (2026-07)

| Endpoint | Quirk |
|---|---|
| `GET /v1/blog/{blog}/subscribers` | **NO trailing slash** (slashed path 404s). Offset-based pagination; `sort_by`/`order` optional. Own blog only. |
| `GET /v1/dialog/` | offset from `extra.offset`, `extra.total` present. |
| `GET /v1/dialog/{id}/message/` | offset = **last message id**, stop on `extra.is_last`. |
| `GET /v1/blog/{blog}/post/` | offset is an opaque string `"{sortOrder}:{intId}"` echoed from `extra.offset` (goes back percent-encoded — server accepts it, live-verified). |
| comments | pagination flags are **chronological**, not directional: terminal page in either order is exactly `is_first && is_last`. The odd-looking loop in `get_all_comments` is CORRECT — do not simplify. |
| `GET /v1/user/current` | not modeled; the account's own blog slug is `blogUrl` in the raw JSON. The bridge should carry the blog slug in config instead of calling this. |
| `POST /v1/dialog/{id}/message/` | sends into an EXISTING dialog only; opening a brand-new dialog with a user is not implemented/verified. |

### 5. Testing layers & commands (Windows host)

| # | Layer | Command |
|---|---|---|
| 1 | static | `cargo fmt --all -- --check` && `cargo clippy --all-targets -- -D warnings` |
| 2 | unit | `cargo test --lib --quiet` |
| 3 | contract (offline, mockito) | `cargo test --tests --quiet` |
| 3b | live canary (manual) | `cargo test --test live_api -- --ignored --nocapture` |
| 4 | dependency audit | `cargo audit` (also enforced by the repo's push gate) |

- `scripts/ci.ps1` runs 1–4; `.claude/hooks/git-gate.ps1` blocks
  commit/push on failures (Claude Code hook; Codex should run
  `scripts/ci.ps1` manually before committing).
- `tests/audit_probes.rs` = regression suite; **each probe documents a
  formerly-real bug**. Keep all 8 green; extend it when fixing anything
  security-shaped.
- Live tests are `#[ignore]`d; anonymous ones use public blogs with small
  limits (be a polite API citizen). Authenticated ones read gitignored
  `.secrets/boosty.json`: `{"access_token", "refresh_token", "device_id"}`
  (dev/test only — see ADR-001). `live_auth_refresh_flow` additionally
  requires env `BOOSTY_TEST_REFRESH=1` because it consumes/rotates the
  stored refresh token and writes the rotated one back.
- Host quirks: cargo is not on PATH — prepend
  `$env:USERPROFILE\.cargo\bin` (PowerShell) per session. PS 5.1
  `Get-Content`/`Set-Content` mangle UTF-8 — use proper file APIs.
- Doctests are NOT covered by layers 1–4; after touching doc examples run
  `cargo test --doc`.

### 6. Model/API-change protocol

Offline fixtures lie (a `price: i32` survived fixtures and broke on live
`0.98`). Any change to models or request construction requires: offline
mockito test + a run of the live canary. Never print tokens in tests; when
distinguishing "model is wrong" from "no access", treat only
`JsonParseDetailed`/`Deserialization` as model errors (see
`tests/live_api.rs::is_model_error`).

---

## Part II — the bridge (to build in vpnctl)

### 7. Decision record — ADR-001, secret storage (2026-07-10)

Credentials (`refresh_token`, `device_id`, optionally an initial
`access_token`) are **injected at runtime from the vpnctl side** — its
existing secret mechanism, or env vars if it has none. Never copy the
`.secrets/*.json` dev pattern to a server. After every sync cycle the
bridge persists the rotated refresh token back to its store (see § 9);
losing a rotated token = manual re-login at boosty.to (F12 → Local Storage
→ key `auth` for tokens, `_clentId` for device id), so treat persistence
as a first-class correctness concern, not a nicety.

### 8. Client construction (required shape)

```rust
use boosty_api::api_client::ApiClient;
use std::time::Duration;

let http = reqwest::Client::builder()
    .connect_timeout(Duration::from_secs(10))
    .timeout(Duration::from_secs(30))      // both timeouts are MANDATORY (§ 2)
    .build()?;
let api = ApiClient::new(http, "https://api.boosty.to");
api.set_refresh_token_and_device_id(&cfg.refresh_token, &cfg.device_id).await?;
```

One `ApiClient` per process is enough — it is `Clone` (shared `Arc` state
inside), and sharing one instance is what makes the single-flight refresh
work.

### 9. Sync loop (the core algorithm)

```text
every SYNC_INTERVAL (suggest 10–15 min; small blog → politeness beats freshness):
  1. subs = api.get_all_subscribers(&cfg.blog_slug, None, None).await
       - on error: log, keep previous state, backoff; NEVER mass-revoke
         because the API was unreachable (fail-safe rule)
  2. desired = subs.filter(|s| s.is_active() && level_grants_vpn(&s.level))
       - s.is_active() == Boosty's own status field ("active"/"inactive");
         it is the authoritative signal, NOT off_time/next_pay_time math
       - level_grants_vpn: config map { boosty_level_id (u64) → vpn_plan },
         keyed on SubscriberLevel.id — names are mutable, ids are not
  3. diff against vpnctl's user store (match key: Subscriber.id (u64);
     store it in vpnctl next to the VPN account; email may be empty — not a key)
       - to_provision: in desired, no VPN access
       - to_revoke:    has VPN access tagged "boosty", not in desired
  4. apply (respect dry_run flag: log actions, change nothing)
  5. persist rotated refresh token:
       if let Some(t) = api.refresh_token().await { if t != stored { store(t) } }
     — after EVERY cycle, success or failure, since any authorized call may
       have refreshed
```

Safety rules (non-negotiable):

- **Fail-safe:** revocation happens only from a *successful, parsed* API
  answer showing the subscriber gone/inactive — never from an error path.
- **Dry-run first:** ship the loop with `dry_run = true` default; flip to
  live only after the user reviews a real diff log.
- **Grace period** on expiry and the exact level→plan map are **OPEN
  QUESTIONS** (§ 12) — build them as config, don't hardcode guesses.
- Don't log tokens (the crate already redacts; keep bridge logs clean of
  `Authorization` headers too). Logging subscriber names/ids is fine —
  it's the blog owner's own data.

### 10. Error handling matrix

| Condition | Meaning | Bridge action |
|---|---|---|
| `ApiError::Unauthorized` after the client's built-in retry | refresh token dead/revoked | alert operator ("re-login needed"), pause provisioning, keep state |
| `AuthError::MissingCredentials` | config never loaded | fail startup loudly |
| `ApiError::HttpStatus { 5xx / 429 }` | server-side / throttled | exponential backoff, keep state |
| `ApiError::HttpRequest` (network/timeout) | transient | same backoff |
| `ApiError::JsonParseDetailed` | **model drift** — Boosty changed a shape | alert + keep state; fix goes into boosty_api with an offline test + live canary (§ 6) |

### 11. Notifications (optional, Phase D)

`api.send_message(dialog_id, &[CommentBlock::text("..."), CommentBlock::text_end()])`
posts a DM. Constraint: only into an **existing** dialog. Find it via
`get_dialogs(...)` matching `dialog.chatmate.id == subscriber.id`; if the
subscriber never opened a dialog, there is nothing to post into — skip,
don't invent an endpoint (opening new dialogs is unverified, § 4).

### 12. OPEN QUESTIONS — ask the user before Phase C

1. Which Boosty subscription level ids grant VPN access, and to which
   vpnctl plan does each map?
2. Expiry semantics: revoke immediately on `status != "active"`, or after
   a grace period (how long)?
3. Revoke = delete the VPN account, or disable it (recoverable)?

### 13. Phased plan with acceptance criteria

| Phase | Deliverable | Done when |
|---|---|---|
| A | config + secrets + client bootstrap in vpnctl | bridge starts, authenticates, fetches subscribers, persists rotated token across a restart |
| B | classification + diff, **dry-run report only** | a scheduled run logs `to_provision`/`to_revoke` correctly against a hand-checked snapshot; unit tests cover the diff on fixture JSON (no live calls in vpnctl CI) |
| C | real provisioning/revocation (needs § 12 answers) | end-to-end: a test subscriber gains access; flipping them inactive (or a doctored fixture) revokes it; fail-safe rule covered by a unit test where the API "errors" and nothing is revoked |
| D | DM notifications on provision | message lands in an existing dialog; absent dialog is skipped without error |

Not needed (YAGNI — do not build): webhook ingestion (Boosty has none
public — polling is the mechanism), a second HTTP client, per-request
token caching layers (the crate already does this), rate-limit middleware
(one small blog; the interval IS the rate limit), zeroize-on-drop.

### 14. Crate quick-reference (signatures the bridge uses)

```rust
// construction / auth
ApiClient::new(client: reqwest::Client, base_url: impl Into<String> + Clone) -> ApiClient
api.set_refresh_token_and_device_id(refresh: &str, device_id: &str) -> ResultAuth<()>
api.set_bearer_token(access: &str) -> ResultAuth<()>          // alternative mode
api.refresh_token() -> Option<String>                          // rotated value; persist it

// subscribers (own blog; authenticated)
api.get_all_subscribers(blog: &str, sort_by: Option<&str>, order: Option<&str>)
    -> ResultApi<Vec<Subscriber>>
Subscriber { id: u64, name: String, email: String /* may be "" */,
             subscribed: bool, status: String, on_time: i64,
             off_time: Option<i64>, next_pay_time: Option<i64>,
             price: f64, payments: f64, level: SubscriberLevel, .. }
Subscriber::is_active(&self) -> bool        // status == "active", authoritative
SubscriberLevel { id: u64, name: String, price: f64,
                  currency_prices: HashMap<String, f64>, deleted: bool,
                  is_hidden: bool, is_archived: bool, .. }

// dialogs / messages (authenticated)
api.get_dialogs(limit: Option<u32>, offset: Option<u64>) -> ResultApi<DialogsResponse>
DialogsResponse { data: Vec<Dialog>, extra: DialogsExtra { offset: u64, total: u64 } }
Dialog { id: u64, chatmate: Chatmate { id: u64, url: String, name: String, .. },
         last_message: Option<Message>, unread_msg_count: u32, .. }
api.get_all_dialog_messages(dialog_id: u64, limit: Option<u32>) -> ResultApi<Vec<Message>>
api.send_message(dialog_id: u64, blocks: &[CommentBlock]) -> ResultApi<Message>
CommentBlock::text(&str) / CommentBlock::text_end() / CommentBlock::smile(&str)

// misc used by tests / diagnostics
api.get_user_subscriptions(limit: Option<u32>, with_follow: Option<bool>)
    -> ResultApi<SubscriptionsResponse>     // the token owner's own subscriptions
```

Error enums live in `boosty_api::error`; models are all importable from
`boosty_api::model::*`.
