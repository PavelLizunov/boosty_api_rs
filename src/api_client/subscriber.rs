use crate::api_client::ApiClient;
use crate::error::ResultApi;
use crate::model::{Subscriber, SubscribersResponse};

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
        let mut path = format!("blog/{blog_name}/subscribers");
        let mut params = Vec::new();
        if let Some(o) = offset {
            params.push(format!("offset={o}"));
        }
        if let Some(l) = limit {
            params.push(format!("limit={l}"));
        }
        if let Some(sb) = sort_by {
            params.push(format!("sort_by={sb}"));
        }
        if let Some(ord) = order {
            params.push(format!("order={ord}"));
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

            let got = resp.data.len();
            all.extend(resp.data);

            if got == 0 || all.len() as u64 >= resp.total {
                break;
            }
        }

        Ok(all)
    }
}
