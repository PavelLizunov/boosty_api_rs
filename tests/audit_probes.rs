//! Regression suite born from the 2026-07 audit: each test pins behavior
//! that was found broken (and has since been fixed) or that encodes
//! live-API semantics which are easy to get wrong.

mod helpers;

use boosty_api::{api_client::ApiClient, error::ApiError};
use mockito::Matcher;
use reqwest::{Client, header::CONTENT_TYPE};
use serde_json::{Value, json};
use std::fs;

use crate::helpers::{api_path, setup};

fn comment_json(int_id: u64) -> Value {
    json!({
        "id": format!("c{int_id}"),
        "intId": int_id,
        "post": { "id": "p" },
        "author": { "id": 1, "name": "u", "hasAvatar": false, "avatarUrl": "" },
        "createdAt": 1_700_000_000u64,
        "isDeleted": false,
        "isBlocked": false,
        "isUpdated": false,
        "replyCount": 0,
        "data": [],
        "reactions": { "dislike":0,"heart":0,"fire":0,"angry":0,"wonder":0,"laught":0,"sad":0,"like":0 },
        "reactionCounters": []
    })
}

/// PROBE 1: DELETE target with 401 must map to ApiError::Unauthorized,
/// like every other endpoint (they all go through handle_response).
#[tokio::test]
async fn probe_delete_target_401_maps_to_unauthorized() {
    let (mut server, base) = setup().await;
    let client = ApiClient::new(Client::new(), &base);

    server
        .mock("DELETE", api_path("target/456").as_str())
        .with_status(401)
        .create_async()
        .await;

    let res = client.delete_blog_target(456).await;
    assert!(
        matches!(&res, Err(ApiError::Unauthorized)),
        "BUG: expected Unauthorized, got {res:?}"
    );
}

/// PROBE 2: DELETE target with HTTP 500 must be an error, not silent success.
#[tokio::test]
async fn probe_delete_target_500_is_an_error() {
    let (mut server, base) = setup().await;
    let client = ApiClient::new(Client::new(), &base);

    server
        .mock("DELETE", api_path("target/456").as_str())
        .with_status(500)
        .with_header(CONTENT_TYPE, "application/json")
        .with_body(r#"{"error":"internal"}"#)
        .create_async()
        .await;

    let res = client.delete_blog_target(456).await;
    assert!(
        res.is_err(),
        "BUG: HTTP 500 on delete returned Ok(()) — server error swallowed"
    );
}

/// PROBE 3: with the refresh flow configured, a 401 triggers exactly one
/// forced refresh + retry (2 GET attempts total), then gives up.
#[tokio::test]
async fn probe_retries_once_on_401_with_refresh_flow() {
    let (mut server, base) = setup().await;
    let client = ApiClient::new(Client::new(), &base);

    client
        .set_refresh_token_and_device_id("r1", "d1")
        .await
        .unwrap();

    server
        .mock("POST", "/oauth/token/")
        .with_status(200)
        .with_header(CONTENT_TYPE, "application/json")
        .with_body(r#"{"access_token":"tokA","refresh_token":"r2","expires_in":3600}"#)
        .create_async()
        .await;

    let get_mock = server
        .mock("GET", api_path("blog/b/post/1").as_str())
        .with_status(401)
        .expect(2) // README-promised behavior: original attempt + retry
        .create_async()
        .await;

    let _ = client.get_post("b", "1").await;
    get_mock.assert_async().await;
}

/// PROBE 4: get_all_comments must stop at the terminal page without an
/// extra empty-page request.
///
/// Live-API flag semantics (verified against api.boosty.to, 2026-07): the
/// flags are chronological. With the default ("top") order every page reads
/// (isFirst=true, isLast=false) until the terminal page, which reads
/// (true, true); the "bottom" order mirrors this. The terminal page in
/// either order is exactly `isFirst && isLast`.
#[tokio::test]
async fn probe_get_all_comments_stops_at_terminal_page() {
    let (mut server, base) = setup().await;
    let client = ApiClient::new(Client::new(), &base);

    let path = api_path("blog/b/post/p/comment/");

    // page 1 (default order): more pages remain -> (isFirst=true, isLast=false)
    server
        .mock("GET", path.as_str())
        .match_query(Matcher::Exact("limit=2".into()))
        .with_status(200)
        .with_header(CONTENT_TYPE, "application/json")
        .with_body(
            json!({
                "data": [comment_json(1000), comment_json(1001)],
                "extra": { "isFirst": true, "isLast": false }
            })
            .to_string(),
        )
        .create_async()
        .await;

    // page 2: terminal page -> (isFirst=true, isLast=true), pagination must stop
    server
        .mock("GET", path.as_str())
        .match_query(Matcher::Exact("offset=1001&limit=2".into()))
        .with_status(200)
        .with_header(CONTENT_TYPE, "application/json")
        .with_body(
            json!({
                "data": [comment_json(2001), comment_json(2002)],
                "extra": { "isFirst": true, "isLast": true }
            })
            .to_string(),
        )
        .create_async()
        .await;

    // past the end: must never be requested
    let extra_page = server
        .mock("GET", path.as_str())
        .match_query(Matcher::Exact("offset=2002&limit=2".into()))
        .with_status(200)
        .with_header(CONTENT_TYPE, "application/json")
        .with_body(
            json!({
                "data": [],
                "extra": { "isFirst": true, "isLast": true }
            })
            .to_string(),
        )
        .expect(0)
        .create_async()
        .await;

    let comments = client
        .get_all_comments("b", "p", Some(2), None, None)
        .await
        .unwrap();
    assert_eq!(comments.len(), 4);
    extra_page.assert_async().await; // fails if a wasted 3rd request happened
}

/// PROBE 5: get_posts must never return more posts than `limit`,
/// even if the server over-delivers.
#[tokio::test]
async fn probe_get_posts_respects_limit() {
    let (mut server, base) = setup().await;
    let client = ApiClient::new(Client::new(), &base);

    let raw = fs::read_to_string("tests/fixtures/api_response_posts.json").unwrap(); // 2 posts

    server
        .mock("GET", api_path("blog/b/post/?limit=1").as_str())
        .with_status(200)
        .with_header(CONTENT_TYPE, "application/json")
        .with_body(raw)
        .create_async()
        .await;

    let posts = client.get_posts("b", 1, None, None).await.unwrap();
    assert!(
        posts.len() <= 1,
        "BUG: asked for limit=1, got {} posts (no client-side truncation)",
        posts.len()
    );
}

/// PROBE 6: Debug output of the client must not leak the bearer token.
#[tokio::test]
async fn probe_debug_does_not_leak_token() {
    let (_server, base) = setup().await;
    let client = ApiClient::new(Client::new(), &base);
    client
        .set_bearer_token("SUPER_SECRET_TOKEN_XYZ")
        .await
        .unwrap();

    let dbg = format!("{client:?}");
    assert!(
        !dbg.contains("SUPER_SECRET_TOKEN_XYZ"),
        "LEAK: Debug output of ApiClient contains the raw token: {dbg}"
    );
}
