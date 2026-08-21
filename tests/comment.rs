use std::fs;

use boosty_api::{api_client::ApiClient, error::ApiError, model::CommentBlock};
use reqwest::{Client, header::CONTENT_TYPE};
use serde_json::{Value, json};

use crate::helpers::{api_path, setup};

mod helpers;

fn comments_page(ids: &[u64], is_first: bool, is_last: bool) -> String {
    let raw = fs::read_to_string("tests/fixtures/api_response_comments_list_page1.json").unwrap();
    let mut response: Value = serde_json::from_str(&raw).unwrap();
    let template = response["data"][0].clone();
    response["data"] = Value::Array(
        ids.iter()
            .map(|id| {
                let mut comment = template.clone();
                comment["id"] = Value::String(format!("comment-{id}"));
                comment["intId"] = json!(id);
                comment
            })
            .collect(),
    );
    response["extra"] = json!({"isFirst": is_first, "isLast": is_last});
    response.to_string()
}

fn assert_pagination(
    result: Result<Vec<boosty_api::model::Comment>, ApiError>,
    reason: &'static str,
) {
    assert!(matches!(
        result,
        Err(ApiError::Pagination {
            resource: "comments",
            reason: actual
        }) if actual == reason
    ));
}

#[tokio::test]
async fn test_create_comment_unauthorized() {
    let (mut server, base) = setup().await;
    let client = ApiClient::new(Client::new(), &base);

    let blog = "blogx";
    let post_id = "pid";
    let path = api_path(&format!("blog/{blog}/post/{post_id}/comment/"));

    server
        .mock("POST", path.as_str())
        .with_status(401)
        .create_async()
        .await;

    let blocks = [CommentBlock::text("hello")];

    let res = client.create_comment(blog, post_id, &blocks, None).await;
    assert!(matches!(res, Err(ApiError::Unauthorized)));
}

#[tokio::test]
async fn test_create_comment_invalid_json() {
    let (mut server, base) = setup().await;
    let client = ApiClient::new(Client::new(), &base);

    let blog = "blog";
    let post_id = "p";
    let path = api_path(&format!("blog/{blog}/post/{post_id}/comment/"));

    server
        .mock("POST", path.as_str())
        .with_status(200)
        .with_header(CONTENT_TYPE, "application/json")
        .with_body("not valid json")
        .create_async()
        .await;

    let blocks = [CommentBlock::text("hi")];

    let res = client.create_comment(blog, post_id, &blocks, None).await;
    assert!(matches!(res, Err(ApiError::JsonParseDetailed { .. })));
}

#[tokio::test]
async fn test_get_comments_response_unauthorized() {
    let (mut server, base) = setup().await;
    let client = ApiClient::new(Client::new(), &base);

    let blog = "b";
    let post_id = "p";
    let path = api_path(&format!("blog/{blog}/post/{post_id}/comment/"));

    server
        .mock("GET", path.as_str())
        .with_status(401)
        .create_async()
        .await;

    let res = client
        .get_comments_response(blog, post_id, None, None, None, None)
        .await;
    assert!(matches!(res, Err(ApiError::Unauthorized)));
}

#[tokio::test]
async fn test_get_comments_response_invalid_json() {
    let (mut server, base) = setup().await;
    let client = ApiClient::new(Client::new(), &base);

    let blog = "b";
    let post_id = "p";
    let path = api_path(&format!("blog/{blog}/post/{post_id}/comment/"));

    server
        .mock("GET", path.as_str())
        .with_status(200)
        .with_header(CONTENT_TYPE, "application/json")
        .with_body("invalid json")
        .create_async()
        .await;

    let res = client
        .get_comments_response(blog, post_id, None, None, None, None)
        .await;
    assert!(matches!(res, Err(ApiError::JsonParseDetailed { .. })));
}

#[tokio::test]
async fn test_create_comment_success() {
    let (mut server, base) = setup().await;
    let client = ApiClient::new(Client::new(), &base);

    let blog = "blog_test";
    let post_id = "post_id_1";

    let comment_resp_body =
        fs::read_to_string("tests/fixtures/api_response_comments.json").unwrap();

    let path = api_path(&format!("blog/{blog}/post/{post_id}/comment/"));

    server
        .mock("POST", path.as_str())
        .with_status(200)
        .with_header(CONTENT_TYPE, "application/json")
        .with_body(comment_resp_body)
        .create_async()
        .await;

    let blocks = vec![
        CommentBlock::text("This is a test comment from file."),
        CommentBlock::text_end(),
    ];

    let reply_id = Some(999);

    let res = client
        .create_comment(blog, post_id, &blocks, reply_id)
        .await;

    assert!(res.is_ok(), "Expected Ok: {:?}", res.err());
    let comment = res.unwrap();

    assert_eq!(
        comment.int_id, 10091879,
        "intId comment not equal to 10091879"
    );
    assert_eq!(
        comment.author.name, "user1",
        "author name not equal to user1"
    );
    assert_eq!(comment.data.len(), 2, "Expected 2 data items");
}

#[tokio::test]
async fn test_get_comments_response_success() {
    let (mut server, base) = setup().await;
    let client = ApiClient::new(Client::new(), &base);

    let blog = "b_list";
    let post_id = "p_list";
    let limit = Some(2);
    let offset = Some(0);

    let path_with_params = api_path(&format!(
        "blog/{blog}/post/{post_id}/comment/?offset=0&limit=2"
    ));

    let comments_resp_body =
        fs::read_to_string("tests/fixtures/api_response_comments_list_page1.json").unwrap();

    server
        .mock("GET", path_with_params.as_str())
        .with_status(200)
        .with_header(CONTENT_TYPE, "application/json")
        .with_body(comments_resp_body)
        .create_async()
        .await;

    let res = client
        .get_comments_response(blog, post_id, limit, None, None, offset)
        .await;

    assert!(res.is_ok(), "Expected Ok, got error: {:?}", res.err());
    let comments_response = res.unwrap();

    assert_eq!(
        comments_response.data.len(),
        2,
        "Expected 2 comments, got {}",
        comments_response.data.len()
    );
    assert_eq!(comments_response.data[0].author.name, "User_A");
    assert_eq!(comments_response.data[1].int_id, 1001);
    assert!(comments_response.extra.is_first, "Expected is_first = true");
    assert!(!comments_response.extra.is_last, "Expected is_last = false");
}

#[tokio::test]
async fn test_get_all_comments_rejects_empty_nonterminal_page() {
    let (mut server, base) = setup().await;
    let client = ApiClient::new(Client::new(), &base);

    server
        .mock("GET", api_path("blog/b/post/p/comment/").as_str())
        .with_status(200)
        .with_header(CONTENT_TYPE, "application/json")
        .with_body(comments_page(&[], false, false))
        .create_async()
        .await;

    assert_pagination(
        client.get_all_comments("b", "p", None, None, None).await,
        "empty nonterminal page",
    );
}

#[tokio::test]
async fn test_get_all_comments_rejects_duplicate_item() {
    let (mut server, base) = setup().await;
    let client = ApiClient::new(Client::new(), &base);

    server
        .mock("GET", api_path("blog/b/post/p/comment/").as_str())
        .with_status(200)
        .with_header(CONTENT_TYPE, "application/json")
        .with_body(comments_page(&[1000], false, false))
        .create_async()
        .await;
    server
        .mock(
            "GET",
            api_path("blog/b/post/p/comment/?offset=1000").as_str(),
        )
        .with_status(200)
        .with_header(CONTENT_TYPE, "application/json")
        .with_body(comments_page(&[1000, 1001], true, true))
        .create_async()
        .await;

    assert_pagination(
        client.get_all_comments("b", "p", None, None, None).await,
        "duplicate item",
    );
}

#[tokio::test]
async fn test_get_all_comments_rejects_stalled_offset() {
    let (mut server, base) = setup().await;
    let client = ApiClient::new(Client::new(), &base);

    server
        .mock("GET", api_path("blog/b/post/p/comment/").as_str())
        .with_status(200)
        .with_header(CONTENT_TYPE, "application/json")
        .with_body(comments_page(&[1000], false, false))
        .create_async()
        .await;
    server
        .mock(
            "GET",
            api_path("blog/b/post/p/comment/?offset=1000").as_str(),
        )
        .with_status(200)
        .with_header(CONTENT_TYPE, "application/json")
        .with_body(comments_page(&[1000], false, false))
        .create_async()
        .await;

    assert_pagination(
        client.get_all_comments("b", "p", None, None, None).await,
        "offset did not advance",
    );
}
