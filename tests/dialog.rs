mod helpers;

use std::fs;

use boosty_api::{api_client::ApiClient, error::ApiError, model::CommentBlock, traits::HasContent};
use mockito::Matcher;
use reqwest::{Client, header::CONTENT_TYPE};

use crate::helpers::{api_path, setup};

#[tokio::test]
async fn test_get_dialogs_success() {
    let (mut server, base) = setup().await;
    let client = ApiClient::new(Client::new(), &base);

    let raw = fs::read_to_string("tests/fixtures/api_response_dialogs.json").unwrap();

    server
        .mock("GET", api_path("dialog/?limit=20").as_str())
        .with_status(200)
        .with_header(CONTENT_TYPE, "application/json")
        .with_body(raw)
        .create_async()
        .await;

    let resp = client.get_dialogs(Some(20), None).await.unwrap();
    assert_eq!(resp.extra.total, 1);
    assert_eq!(resp.data.len(), 1);
    let d = &resp.data[0];
    assert_eq!(d.id, 3323943);
    assert_eq!(d.chatmate.name, "El-Kuzari");
    let last = d.last_message.as_ref().expect("last message present");
    assert_eq!(last.author_id, 7597253);
    assert_eq!(last.attachments.text.count, 1);
}

#[tokio::test]
async fn test_get_dialogs_unauthorized() {
    let (mut server, base) = setup().await;
    let client = ApiClient::new(Client::new(), &base);

    server
        .mock("GET", api_path("dialog/?limit=5").as_str())
        .with_status(401)
        .create_async()
        .await;

    let res = client.get_dialogs(Some(5), None).await;
    assert!(matches!(res, Err(ApiError::Unauthorized)));
}

#[tokio::test]
async fn test_get_dialog_messages_and_content() {
    let (mut server, base) = setup().await;
    let client = ApiClient::new(Client::new(), &base);

    let raw = fs::read_to_string("tests/fixtures/api_response_messages.json").unwrap();

    server
        .mock("GET", api_path("dialog/3323943/message/?limit=30").as_str())
        .with_status(200)
        .with_header(CONTENT_TYPE, "application/json")
        .with_body(raw)
        .create_async()
        .await;

    let resp = client
        .get_dialog_messages(3323943, Some(30), None)
        .await
        .unwrap();
    assert_eq!(resp.data.len(), 2);
    assert!(resp.extra.is_last);

    // Content extraction reuses MediaData: text + link + text.
    let content = resp.data[0].extract_content();
    assert_eq!(content.len(), 3);
}

#[tokio::test]
async fn test_get_all_dialog_messages_stops_at_is_last() {
    let (mut server, base) = setup().await;
    let client = ApiClient::new(Client::new(), &base);

    let raw = fs::read_to_string("tests/fixtures/api_response_messages.json").unwrap();

    // Single page marked isLast=true; a second (offset) request must not happen.
    server
        .mock("GET", api_path("dialog/42/message/?limit=30").as_str())
        .with_status(200)
        .with_header(CONTENT_TYPE, "application/json")
        .with_body(raw)
        .create_async()
        .await;

    let extra = server
        .mock(
            "GET",
            Matcher::Regex(r"^/v1/dialog/42/message/\?offset=.*$".into()),
        )
        .with_status(200)
        .with_header(CONTENT_TYPE, "application/json")
        .with_body(r#"{"data":[],"extra":{"isLast":true,"offset":0}}"#)
        .expect(0)
        .create_async()
        .await;

    let messages = client.get_all_dialog_messages(42, Some(30)).await.unwrap();
    assert_eq!(messages.len(), 2);
    extra.assert_async().await;
}

#[tokio::test]
async fn test_send_message_success() {
    let (mut server, base) = setup().await;
    let client = ApiClient::new(Client::new(), &base);

    // The response echoes a single created message; reuse the messages fixture's
    // first element shape via a minimal inline body.
    let body = r#"{
        "id": 99999,
        "dialogId": 3323943,
        "createdAt": 1783600000,
        "authorId": 35206396,
        "isRead": false,
        "isPaid": false,
        "isDeleted": false,
        "isFeePaid": false,
        "price": 0,
        "previewType": "text",
        "currencyPrices": { "RUB": 0, "USD": 0 },
        "teaser": [],
        "data": [ { "type": "text", "modificator": "", "content": "[\"hi\",\"unstyled\",[]]" } ],
        "attachments": {
            "text": { "count": 1 }, "files": { "count": 0 }, "audios": { "count": 0 },
            "images": { "count": 0, "previewUrl": "" }, "videos": { "count": 0, "previewUrl": "" }
        }
    }"#;

    server
        .mock("POST", api_path("dialog/3323943/message/").as_str())
        .match_header(
            "content-type",
            Matcher::Regex("multipart/form-data.*".into()),
        )
        .with_status(200)
        .with_header(CONTENT_TYPE, "application/json")
        .with_body(body)
        .create_async()
        .await;

    let blocks = [CommentBlock::text("hi"), CommentBlock::text_end()];
    let msg = client.send_message(3323943, &blocks).await.unwrap();
    assert_eq!(msg.id, 99999);
    assert_eq!(msg.dialog_id, 3323943);
}

#[tokio::test]
async fn test_send_message_unauthorized() {
    let (mut server, base) = setup().await;
    let client = ApiClient::new(Client::new(), &base);

    server
        .mock("POST", api_path("dialog/1/message/").as_str())
        .with_status(401)
        .create_async()
        .await;

    let res = client.send_message(1, &[CommentBlock::text("x")]).await;
    assert!(matches!(res, Err(ApiError::Unauthorized)));
}

#[tokio::test]
async fn test_get_dialogs_invalid_json() {
    let (mut server, base) = setup().await;
    let client = ApiClient::new(Client::new(), &base);

    server
        .mock("GET", api_path("dialog/").as_str())
        .with_status(200)
        .with_header(CONTENT_TYPE, "application/json")
        .with_body("not json")
        .create_async()
        .await;

    let res = client.get_dialogs(None, None).await;
    assert!(matches!(res, Err(ApiError::JsonParseDetailed { .. })));
}
