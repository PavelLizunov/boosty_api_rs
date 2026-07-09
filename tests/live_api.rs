//! Live contract tests against the real Boosty API (public, anonymous data).
//!
//! Ignored by default so the regular suite stays offline and deterministic.
//! Run manually to verify the models still match the live API:
//!
//! ```text
//! cargo test --test live_api -- --ignored --nocapture
//! ```

use boosty_api::api_client::ApiClient;
use boosty_api::traits::HasContent;
use reqwest::Client;

const BASE_URL: &str = "https://api.boosty.to";

/// Public blogs with anonymously visible posts (checked 2026-07).
const BLOGS: &[&str] = &["boosty", "ixbtgames", "uebermarginal", "stopgame"];

fn client() -> ApiClient {
    ApiClient::new(Client::new(), BASE_URL)
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
