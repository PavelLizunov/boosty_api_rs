use crate::api_client::{ApiClient, DEFAULT_PAGE_SIZE, encode_segment};
use crate::error::{ApiError, ResultApi};
use crate::model::{Post, PostsResponse};
use std::collections::HashSet;

impl ApiClient {
    /// Get a single post once, without automatic retry on "not available" or HTTP 401.
    ///
    /// # Parameters
    ///
    /// - `blog_name`: identifier or name of the blog.
    /// - `post_id`: identifier of the post.
    ///
    /// # Returns
    ///
    /// On success, returns the `Post` object.
    ///
    /// # Errors
    ///
    /// - `ApiError::Unauthorized` if the HTTP status is 401 Unauthorized.
    /// - `ApiError::HttpStatus` for other non-success HTTP statuses, with status and endpoint info.
    /// - `ApiError::HttpRequest` if the HTTP request fails.
    /// - `ApiError::JsonParseDetailed` if the response body cannot be parsed into a `Post`.
    pub async fn get_post(&self, blog_name: &str, post_id: &str) -> ResultApi<Post> {
        let path = format!(
            "blog/{}/post/{}",
            encode_segment(blog_name),
            encode_segment(post_id)
        );

        let response = self.get_request(&path).await?;
        let response = self.handle_response(&path, response).await?;

        self.parse_json(response).await
    }

    /// Get multiple posts for a blog.
    ///
    /// # Parameters
    ///
    /// - `blog_name`: blog identifier/name.
    /// - `limit`: maximum number of posts to return.
    /// - `page_size`: number of posts to fetch per page. Defaults to 20.
    /// - `start_offset`: offset to start fetching posts from. Defaults from first post.
    ///
    /// # Returns
    ///
    /// On success, returns at most `limit` `Post` items.
    ///
    /// # Errors
    ///
    /// - `ApiError::Unauthorized` if the HTTP status is 401 Unauthorized.
    /// - `ApiError::HttpStatus` for other non-success HTTP statuses.
    /// - `ApiError::HttpRequest` if the HTTP request fails.
    /// - `ApiError::JsonParseDetailed` if a response body cannot be parsed into a `PostsResponse`.
    pub async fn get_posts(
        &self,
        blog_name: &str,
        limit: usize,
        page_size: Option<usize>,
        start_offset: Option<String>,
    ) -> ResultApi<Vec<Post>> {
        if limit == 0 {
            return Ok(Vec::new());
        }

        let page_size = page_size.unwrap_or(DEFAULT_PAGE_SIZE);

        let mut all_posts = Vec::new();
        let mut seen = HashSet::new();
        let mut offset = start_offset;

        loop {
            let current_limit = page_size.min(limit - all_posts.len());
            let mut path = format!(
                "blog/{}/post/?limit={current_limit}",
                encode_segment(blog_name)
            );
            // The offset string is echoed back from the previous response —
            // server data, so it gets encoded like any other input.
            if let Some(ref off) = offset {
                path.push_str(&format!("&offset={}", encode_segment(off)));
            }

            let response = self.get_request(&path).await?;
            let response = self.handle_response(&path, response).await?;

            let posts_response: PostsResponse = self.parse_json(response).await?;

            let data_len = posts_response.data.len();
            for post in posts_response.data {
                if !seen.insert(post.id.clone()) {
                    return Err(ApiError::Pagination {
                        resource: "posts",
                        reason: "duplicate item",
                    });
                }
                all_posts.push(post);
            }

            if posts_response.extra.is_last || all_posts.len() >= limit {
                break;
            }

            if data_len == 0 {
                return Err(ApiError::Pagination {
                    resource: "posts",
                    reason: "empty nonterminal page",
                });
            }

            let next_offset = Some(posts_response.extra.offset);
            if next_offset == offset {
                return Err(ApiError::Pagination {
                    resource: "posts",
                    reason: "offset did not advance",
                });
            }
            offset = next_offset;
        }

        // The server may over-deliver; never return more than asked for.
        all_posts.truncate(limit);
        Ok(all_posts)
    }
}
