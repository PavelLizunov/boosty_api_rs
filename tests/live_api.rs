//! Live contract tests against the real Boosty API (public, anonymous data).
//!
//! Ignored by default so the regular suite stays offline and deterministic.
//! Run manually to verify the models still match the live API:
//!
//! ```text
//! cargo test --test live_api -- --ignored --nocapture
//! ```

use boosty_api::api_client::ApiClient;
use boosty_api::error::ApiError;
use boosty_api::traits::HasContent;
use reqwest::Client;
use serde::Deserialize;

const BASE_URL: &str = "https://api.boosty.to";

/// Public blogs with anonymously visible posts (checked 2026-07).
const BLOGS: &[&str] = &["boosty", "ixbtgames", "uebermarginal", "stopgame"];

fn client() -> ApiClient {
    ApiClient::new(Client::new(), BASE_URL)
}

/// Credentials for the authenticated live tests, read from the gitignored
/// `.secrets/boosty.json` (see CLAUDE.md § 8 for how to obtain them).
/// Values are NEVER printed.
#[derive(Deserialize)]
struct Secrets {
    #[serde(default)]
    access_token: String,
    #[serde(default)]
    refresh_token: String,
    #[serde(default)]
    device_id: String,
}

fn load_secrets() -> Option<Secrets> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/.secrets/boosty.json");
    let raw = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

/// Authorized client via static bearer token, or None (test should skip).
async fn authorized_client() -> Option<ApiClient> {
    let secrets = load_secrets()?;
    if secrets.access_token.is_empty() {
        return None;
    }
    let client = client();
    client.set_bearer_token(&secrets.access_token).await.ok()?;
    Some(client)
}

/// Resolve the token owner's own blog slug via a raw `/user/current` call
/// (the crate doesn't model the full profile; the test only needs blogUrl).
async fn own_blog_url() -> Option<String> {
    let secrets = load_secrets()?;
    if secrets.access_token.is_empty() {
        return None;
    }
    #[derive(Deserialize)]
    struct Current {
        #[serde(rename = "blogUrl")]
        blog_url: String,
    }
    Client::new()
        .get(format!("{BASE_URL}/v1/user/current"))
        .bearer_auth(&secrets.access_token)
        .header("User-Agent", "Mozilla/5.0")
        .send()
        .await
        .ok()?
        .json::<Current>()
        .await
        .ok()
        .map(|c| c.blog_url)
        .filter(|u| !u.is_empty())
}

/// Distinguish "model is wrong" (must fail the test) from "no access /
/// endpoint quirk" (fine on foreign blogs).
fn is_model_error(e: &ApiError) -> bool {
    matches!(
        e,
        ApiError::JsonParseDetailed { .. } | ApiError::Deserialization(_)
    )
}

#[tokio::test]
#[ignore = "hits the live Boosty API"]
async fn live_posts_parse() {
    let client = client();
    let mut errors = Vec::new();

    for blog in BLOGS {
        match client.get_posts(blog, 40, Some(20), None).await {
            Ok(posts) => {
                println!("{blog}: parsed {} posts", posts.len());
                // Content extraction must not panic on any real post.
                for post in &posts {
                    let _ = post.extract_content();
                }
            }
            Err(e) => errors.push(format!("{blog}: {e}")),
        }
    }

    assert!(errors.is_empty(), "live post parse failures:\n{errors:#?}");
}

#[tokio::test]
#[ignore = "hits the live Boosty API"]
async fn live_comments_parse() {
    let client = client();
    let mut errors = Vec::new();
    let mut checked = 0usize;

    for blog in BLOGS {
        let posts = match client.get_posts(blog, 20, None, None).await {
            Ok(p) => p,
            Err(e) => {
                errors.push(format!("{blog} (posts): {e}"));
                continue;
            }
        };

        // Parse comments (incl. replies) for a couple of commented posts per
        // blog. Posts without anonymous access answer 403 for comments.
        for post in posts
            .iter()
            .filter(|p| p.has_access && p.count.comments > 0)
            .take(2)
        {
            match client
                .get_all_comments(blog, &post.id, Some(20), Some(5), None)
                .await
            {
                Ok(comments) => {
                    checked += 1;
                    println!("{blog}/{}: parsed {} comments", post.id, comments.len());
                }
                Err(e) => errors.push(format!("{blog}/{} (comments): {e}", post.id)),
            }
        }
    }

    println!("checked comments on {checked} posts");
    assert!(
        errors.is_empty(),
        "live comment parse failures:\n{errors:#?}"
    );
}

/// AUTH: current user's subscriptions — the only live coverage for the
/// Subscription/BlogInfo/SubscriptionLevelInfo models.
#[tokio::test]
#[ignore = "hits the live Boosty API; needs .secrets/boosty.json"]
async fn live_auth_user_subscriptions_parse() {
    let Some(client) = authorized_client().await else {
        println!("SKIPPED: no access_token in .secrets/boosty.json");
        return;
    };

    match client.get_user_subscriptions(Some(50), Some(true)).await {
        Ok(subs) => {
            println!(
                "parsed {} subscriptions (total={})",
                subs.data.len(),
                subs.total
            );
        }
        Err(e) => panic!("user subscriptions failed: {e}"),
    }
}

/// AUTH: posts of subscribed blogs — exercises paid posts with full media
/// (ok_video player urls, audio, files) that anonymous fetches never see.
#[tokio::test]
#[ignore = "hits the live Boosty API; needs .secrets/boosty.json"]
async fn live_auth_subscribed_posts_and_bundles_parse() {
    let Some(client) = authorized_client().await else {
        println!("SKIPPED: no access_token in .secrets/boosty.json");
        return;
    };

    let subs = match client.get_user_subscriptions(Some(10), Some(true)).await {
        Ok(s) => s,
        Err(e) => panic!("user subscriptions failed: {e}"),
    };

    let mut errors = Vec::new();
    for sub in &subs.data {
        let blog = sub.blog.blog_url.as_str();

        match client.get_posts(blog, 20, None, None).await {
            Ok(posts) => {
                let accessible = posts.iter().filter(|p| p.has_access).count();
                println!(
                    "{blog}: parsed {} posts ({accessible} accessible)",
                    posts.len()
                );
                for post in &posts {
                    let _ = post.extract_content();
                }
            }
            Err(e) if is_model_error(&e) => errors.push(format!("{blog} (posts): {e}")),
            Err(e) => println!("{blog}: posts not readable ({e}) — not a model error"),
        }

        match client.get_bundles(blog).await {
            Ok(b) => println!("{blog}: parsed {} bundles", b.data.bundles.len()),
            Err(e) if is_model_error(&e) => errors.push(format!("{blog} (bundles): {e}")),
            Err(e) => println!("{blog}: bundles not readable ({e}) — not a model error"),
        }
    }

    assert!(errors.is_empty(), "live auth parse failures:\n{errors:#?}");
}

/// AUTH: dialogs + messages — the only live coverage for the
/// Dialog/Chatmate/Message models.
#[tokio::test]
#[ignore = "hits the live Boosty API; needs .secrets/boosty.json"]
async fn live_auth_dialogs_and_messages_parse() {
    let Some(client) = authorized_client().await else {
        println!("SKIPPED: no access_token in .secrets/boosty.json");
        return;
    };

    let dialogs = match client.get_dialogs(Some(20), None).await {
        Ok(d) => d,
        Err(e) => panic!("dialogs failed: {e}"),
    };
    println!(
        "parsed {} dialogs (total={})",
        dialogs.data.len(),
        dialogs.extra.total
    );

    let mut checked = 0usize;
    for dialog in dialogs.data.iter().take(5) {
        match client.get_all_dialog_messages(dialog.id, Some(30)).await {
            Ok(messages) => {
                checked += 1;
                // Content extraction must not panic on any real message.
                for m in &messages {
                    let _ = m.extract_content();
                }
                println!(
                    "dialog {} with {}: parsed {} messages",
                    dialog.id,
                    dialog.chatmate.name,
                    messages.len()
                );
            }
            Err(e) => panic!("messages for dialog {} failed: {e}", dialog.id),
        }
    }
    println!("checked messages in {checked} dialogs");
}

/// AUTH: own-blog subscribers — the only live coverage for the
/// Subscriber/SubscriberLevel models. Skips gracefully if the account
/// has no blog / no subscribers.
#[tokio::test]
#[ignore = "hits the live Boosty API; needs .secrets/boosty.json"]
async fn live_auth_subscribers_parse() {
    let Some(client) = authorized_client().await else {
        println!("SKIPPED: no access_token in .secrets/boosty.json");
        return;
    };

    // The subscribers endpoint is for the caller's OWN blog; resolve it.
    let Some(blog) = own_blog_url().await else {
        println!("SKIPPED: could not resolve own blog url");
        return;
    };

    match client
        .get_all_subscribers(&blog, Some("on_time"), Some("gt"))
        .await
    {
        Ok(subs) => {
            for s in &subs {
                // touch nested level so a bad SubscriberLevel would surface
                let _ = s.level.currency_prices.len();
            }
            println!("{blog}: parsed {} subscribers", subs.len());
        }
        Err(ApiError::HttpStatus { status, .. }) if status.as_u16() == 404 => {
            println!("SKIPPED: {blog} has no subscribers endpoint (not a blogger?)");
        }
        Err(e) => panic!("subscribers failed: {e}"),
    }
}

/// AUTH + OPT-IN: real refresh flow. CONSUMES the stored refresh token
/// (Boosty rotates it) and writes the rotated one back to
/// `.secrets/boosty.json`. The browser session that produced the token may
/// need a re-login afterwards, so this requires BOOSTY_TEST_REFRESH=1.
#[tokio::test]
#[ignore = "hits the live Boosty API; rotates the refresh token"]
async fn live_auth_refresh_flow() {
    if std::env::var("BOOSTY_TEST_REFRESH").as_deref() != Ok("1") {
        println!("SKIPPED: set BOOSTY_TEST_REFRESH=1 to run (rotates the refresh token)");
        return;
    }
    let Some(secrets) = load_secrets() else {
        println!("SKIPPED: no .secrets/boosty.json");
        return;
    };
    if secrets.refresh_token.is_empty() || secrets.device_id.is_empty() {
        println!("SKIPPED: refresh_token/device_id missing in .secrets/boosty.json");
        return;
    }

    let client = client();
    client
        .set_refresh_token_and_device_id(&secrets.refresh_token, &secrets.device_id)
        .await
        .unwrap();

    // Any authorized call triggers the refresh; user/subscriptions is read-only.
    client
        .get_user_subscriptions(Some(1), None)
        .await
        .expect("refresh flow + authorized request failed");

    let rotated = client
        .refresh_token()
        .await
        .expect("refresh token must be present after refresh");
    assert_ne!(rotated, secrets.refresh_token, "token was not rotated");

    // Persist the rotated token so the credentials stay usable.
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/.secrets/boosty.json");
    let updated = serde_json::json!({
        "access_token": secrets.access_token,
        "refresh_token": rotated,
        "device_id": secrets.device_id,
    });
    std::fs::write(path, serde_json::to_string_pretty(&updated).unwrap()).unwrap();
    println!("refresh flow OK; rotated token persisted back to .secrets/boosty.json");
}

#[tokio::test]
#[ignore = "hits the live Boosty API"]
async fn live_subscription_levels_and_targets_parse() {
    let client = client();
    let mut errors = Vec::new();

    for blog in BLOGS {
        match client.get_blog_subscription_levels(blog, Some(true)).await {
            Ok(levels) => println!("{blog}: parsed {} subscription levels", levels.data.len()),
            Err(e) => errors.push(format!("{blog} (levels): {e}")),
        }

        match client.get_blog_targets(blog).await {
            Ok(targets) => println!("{blog}: parsed {} targets", targets.data.len()),
            Err(e) => errors.push(format!("{blog} (targets): {e}")),
        }
    }

    assert!(errors.is_empty(), "live parse failures:\n{errors:#?}");
}
