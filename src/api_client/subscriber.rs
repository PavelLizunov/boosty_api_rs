use crate::api_client::{ApiClient, encode_segment};
use crate::error::{ApiError, ResultApi};
use crate::model::{Subscriber, SubscribersResponse};
use std::collections::HashSet;

/// Per-request page size used by [`ApiClient::get_all_subscribers`].
const SUBSCRIBERS_PAGE_SIZE: u32 = 100;

impl ApiClient {
    /// Get a page of the blog's subscribers.
    ///
    /// Requires an authenticated client that owns the blog.
    ///
    /// # Arguments
    ///
    /// * `blog_name` - blog url/slug (usually the current user's own blog).
    /// * `limit` - max subscribers to return (optional).
    /// * `offset` - pagination offset (optional).
    /// * `sort_by` - sort field, e.g. `"on_time"` (optional).
    /// * `order` - sort order, e.g. `"gt"` / `"lt"` (optional).
    ///
    /// # Errors
    ///
    /// - `ApiError::Unauthorized` if the HTTP status is 401 Unauthorized.
    /// - `ApiError::HttpStatus` for other non-success HTTP statuses.
    /// - `ApiError::HttpRequest` if the HTTP request fails.
    /// - `ApiError::JsonParseDetailed` if the body cannot be parsed into a `SubscribersResponse`.
    pub async fn get_subscribers(
        &self,
        blog_name: &str,
        limit: Option<u32>,
        offset: Option<u64>,
        sort_by: Option<&str>,
        order: Option<&str>,
    ) -> ResultApi<SubscribersResponse> {
        let mut path = format!("blog/{}/subscribers", encode_segment(blog_name));
        let mut params = Vec::new();
        if let Some(o) = offset {
            params.push(format!("offset={o}"));
        }
        if let Some(l) = limit {
            params.push(format!("limit={l}"));
        }
        if let Some(sb) = sort_by {
            params.push(format!("sort_by={}", encode_segment(sb)));
        }
        if let Some(ord) = order {
            params.push(format!("order={}", encode_segment(ord)));
        }
        if !params.is_empty() {
            path.push('?');
            path.push_str(&params.join("&"));
        }

        let response = self.get_request(&path).await?;
        let response = self.handle_response(&path, response).await?;

        self.parse_json(response).await
    }

    /// Get all subscribers of the blog, following offset pagination.
    ///
    /// # Arguments
    ///
    /// * `blog_name` - blog url/slug.
    /// * `sort_by` - sort field, e.g. `"on_time"` (optional).
    /// * `order` - sort order, e.g. `"gt"` (optional).
    ///
    /// # Errors
    ///
    /// Same as [`ApiClient::get_subscribers`].
    pub async fn get_all_subscribers(
        &self,
        blog_name: &str,
        sort_by: Option<&str>,
        order: Option<&str>,
    ) -> ResultApi<Vec<Subscriber>> {
        let mut all = Vec::new();
        let mut seen = HashSet::new();
        let mut expected_total = None;

        loop {
            let resp = self
                .get_subscribers(
                    blog_name,
                    Some(SUBSCRIBERS_PAGE_SIZE),
                    Some(all.len() as u64),
                    sort_by,
                    order,
                )
                .await?;

            let total = *expected_total.get_or_insert(resp.total);
            if resp.total != total {
                return Err(ApiError::Pagination {
                    resource: "subscribers",
                    reason: "total changed between pages",
                });
            }

            if resp.data.is_empty() {
                if all.len() as u64 == total {
                    return Ok(all);
                }
                return Err(ApiError::Pagination {
                    resource: "subscribers",
                    reason: "empty page before expected total",
                });
            }

            for subscriber in resp.data {
                if !seen.insert(subscriber.id) {
                    return Err(ApiError::Pagination {
                        resource: "subscribers",
                        reason: "duplicate item",
                    });
                }
                all.push(subscriber);
            }

            match (all.len() as u64).cmp(&total) {
                std::cmp::Ordering::Equal => return Ok(all),
                std::cmp::Ordering::Greater => {
                    return Err(ApiError::Pagination {
                        resource: "subscribers",
                        reason: "page exceeded expected total",
                    });
                }
                std::cmp::Ordering::Less => {}
            }
        }
    }
}
