mod helpers;

use std::fs;

use boosty_api::{api_client::ApiClient, error::ApiError};
use reqwest::{Client, header::CONTENT_TYPE};

use crate::helpers::{api_path, setup};

#[tokio::test]
async fn test_get_subscribers_success() {
    let (mut server, base) = setup().await;
    let client = ApiClient::new(Client::new(), &base);

    let blog = "ninitux";
    let raw = fs::read_to_string("tests/fixtures/api_response_subscribers.json").unwrap();

    server
        .mock(
            "GET",
            api_path(&format!(
                "blog/{blog}/subscribers?limit=5&sort_by=on_time&order=gt"
            ))
            .as_str(),
        )
        .with_status(200)
        .with_header(CONTENT_TYPE, "application/json")
        .with_body(raw)
        .create_async()
        .await;

    let resp = client
        .get_subscribers(blog, Some(5), None, Some("on_time"), Some("gt"))
        .await
        .unwrap();

    assert_eq!(resp.total, 2);
    assert_eq!(resp.data.len(), 2);

    let first = &resp.data[0];
    assert_eq!(first.id, 40592781);
    assert_eq!(first.email, "user@example.com");
    assert_eq!(first.level.name, "Follower");
    assert!(first.off_time.is_none());

    // Second subscriber exercises empty email, fractional price, and the
    // optional level fields (parentId present).
    let second = &resp.data[1];
    assert_eq!(second.email, "");
    assert_eq!(second.price, 300.5);
    assert_eq!(second.off_time, Some(1790000000));
    assert_eq!(second.level.parent_id, Some(2990987));
    assert!(second.level.flags.is_limited);

    // Classification the provisioning bridge keys on.
    assert!(first.is_active(), "status=active should be active");
    assert!(!second.is_active(), "status=inactive should not be active");
}

#[tokio::test]
async fn test_get_all_subscribers_paginates() {
    let (mut server, base) = setup().await;
    let client = ApiClient::new(Client::new(), &base);

    let blog = "ninitux";

    // total=3, page size 100 → offset=0 returns 2, offset=2 returns the last 1.
    let page1 = r#"{"data":[
        {"id":1,"name":"A","email":"","hasAvatar":false,"avatarUrl":"","isOfficial":false,"isBlackListed":false,"isFeePaid":false,"canWrite":true,"subscribed":true,"status":"active","onTime":1,"price":0,"payments":0,
         "level":{"id":1,"name":"L","price":0,"currencyPrices":{},"createdAt":1,"ownerId":1,"deleted":false,"isHidden":false,"isLimited":false,"isArchived":false,"flags":{"isHidden":false,"isLimited":false,"isArchived":false},"data":[]}},
        {"id":2,"name":"B","email":"","hasAvatar":false,"avatarUrl":"","isOfficial":false,"isBlackListed":false,"isFeePaid":false,"canWrite":true,"subscribed":true,"status":"active","onTime":1,"price":0,"payments":0,
         "level":{"id":1,"name":"L","price":0,"currencyPrices":{},"createdAt":1,"ownerId":1,"deleted":false,"isHidden":false,"isLimited":false,"isArchived":false,"flags":{"isHidden":false,"isLimited":false,"isArchived":false},"data":[]}}
    ],"total":3,"limit":100,"offset":0}"#;

    let page2 = r#"{"data":[
        {"id":3,"name":"C","email":"","hasAvatar":false,"avatarUrl":"","isOfficial":false,"isBlackListed":false,"isFeePaid":false,"canWrite":true,"subscribed":true,"status":"active","onTime":1,"price":0,"payments":0,
         "level":{"id":1,"name":"L","price":0,"currencyPrices":{},"createdAt":1,"ownerId":1,"deleted":false,"isHidden":false,"isLimited":false,"isArchived":false,"flags":{"isHidden":false,"isLimited":false,"isArchived":false},"data":[]}}
    ],"total":3,"limit":100,"offset":2}"#;

    server
        .mock(
            "GET",
            api_path(&format!("blog/{blog}/subscribers?offset=0&limit=100")).as_str(),
        )
        .with_status(200)
        .with_header(CONTENT_TYPE, "application/json")
        .with_body(page1)
        .create_async()
        .await;

    server
        .mock(
            "GET",
            api_path(&format!("blog/{blog}/subscribers?offset=2&limit=100")).as_str(),
        )
        .with_status(200)
        .with_header(CONTENT_TYPE, "application/json")
        .with_body(page2)
        .create_async()
        .await;

    let all = client.get_all_subscribers(blog, None, None).await.unwrap();
    assert_eq!(all.len(), 3);
    assert_eq!(all[2].id, 3);
}

#[tokio::test]
async fn test_get_subscribers_unauthorized() {
    let (mut server, base) = setup().await;
    let client = ApiClient::new(Client::new(), &base);

    server
        .mock("GET", api_path("blog/b/subscribers?limit=5").as_str())
        .with_status(401)
        .create_async()
        .await;

    let res = client.get_subscribers("b", Some(5), None, None, None).await;
    assert!(matches!(res, Err(ApiError::Unauthorized)));
}

#[tokio::test]
async fn test_get_subscribers_invalid_json() {
    let (mut server, base) = setup().await;
    let client = ApiClient::new(Client::new(), &base);

    server
        .mock("GET", api_path("blog/b/subscribers").as_str())
        .with_status(200)
        .with_header(CONTENT_TYPE, "application/json")
        .with_body("not json")
        .create_async()
        .await;

    let res = client.get_subscribers("b", None, None, None, None).await;
    assert!(matches!(res, Err(ApiError::JsonParseDetailed { .. })));
}
