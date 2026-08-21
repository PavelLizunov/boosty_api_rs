# Boosty API client — implementation contract

This repository is a small async Rust client for Boosty. It is an HTTP and
authentication boundary, not an application and not a bridge to another
product. Product-specific policy belongs in the consuming repository.

## Scope

- One `ApiClient` with endpoint modules and typed response models.
- `reqwest` with rustls; no native TLS dependency.
- Read and mutation requests share authorization and response handling.
- Offline contract tests pin every supported response or request shape.

## Authentication

`ApiClient` supports mutually exclusive static-bearer and refresh-token modes.
Long-running processes must configure refresh mode with
`set_refresh_token_and_device_id_with_persister`. Boosty can rotate the refresh
token during any refresh. The callback runs under the authentication lock and
must durably store the `(refresh_token, device_id)` pair before the refreshed
access token can authorize the original request. If persistence fails, the
request fails closed with `AuthError::TokenPersist`.

The legacy `set_refresh_token_and_device_id` method remains for short-lived or
compatibility callers, but it does not provide crash-safe rotation persistence.
Reading `refresh_token()` and saving it after a request is not a safe substitute:
the process can stop after rotation and before that later save.

The callback is synchronous by design. A service implementation should use a
service-owned directory and file, restrictive permissions, a temporary file,
file sync, atomic rename, and directory sync. It must never log tokens or raw
I/O errors that can expose paths.

The refresh HTTP call holds a mutex to prevent refresh stampedes. Consumers must
therefore configure both connection and request timeouts on the supplied
`reqwest::Client`.

## Request and retry policy

Every endpoint goes through `send_authorized`, `handle_response`, and
`parse_json`. String path and query segments go through `encode_segment`.

- GET requests may refresh and retry once after a 401.
- POST, PUT, DELETE, and multipart requests are never retried by the SDK after
  transport. A caller cannot know whether the remote mutation took effect.
- Other HTTP failures are not retried.

Applications must journal mutations before dispatch and reconcile them with an
independent read rather than blindly retrying an uncertain write.

## Collection completeness

Helpers that aggregate pages fail closed when they cannot prove progress:

- subscribers freeze the first page's total and reject changed totals,
  duplicates, early empty pages, and overshoot;
- posts, comments, and dialog messages reject duplicate items and a repeated
  nonterminal offset;
- empty nonterminal pages are errors.

Callers must preserve their last known-good state on `ApiError::Pagination`.

## Verification

Run the full offline gate before every commit:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\ci.ps1
```

After changing documentation examples, also run:

```powershell
cargo test --doc
```

The ignored live canary requires separately authorized credentials and must
never run in normal CI. Model or request-shape changes need both an offline
contract test and an explicitly authorized live canary.
