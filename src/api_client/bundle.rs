use crate::{
    api_client::{ApiClient, encode_segment},
    error::{ApiError, ResultApi},
    model::{BundleItemsResponse, BundleQuery, BundlesResponse},
};

impl ApiClient {
    /// Get all bundles for a blog.
    ///
    /// # Arguments
    /// * `blog_name` - Blog name
    ///
    /// # Returns
    /// * On success, returns a `BundlesResponse` containing the `bundles` field with `Bundle` items.
    ///
    /// # Errors
    /// * `ApiError::Unauthorized` if the HTTP status is 401 Unauthorized.
    /// * `ApiError::HttpStatus` for other non-success HTTP statuses, with status and endpoint info.
    /// * `ApiError::HttpRequest` if the HTTP request fails.
    /// * `ApiError::JsonParseDetailed` if the response body cannot be parsed into a `BundlesResponse`.
    pub async fn get_bundles(&self, blog_name: &str) -> ResultApi<BundlesResponse> {
        let path = format!("blog/{}/bundle/", encode_segment(blog_name));

        let response = self.get_request(&path).await?;
        let response = self.handle_response(&path, response).await?;

        self.parse_json(response).await
    }

    /// Get posts within a specific bundle.
    ///
    /// # Arguments
    /// * `blog_name` - Blog name
    /// * `bundle_id` - Bundle UUID
    /// * `query` - Bundle query
    ///
    /// # Returns
    /// * On success, returns a `BundleItemsResponse` containing the `bundleItems` field with `BundleItem` items.
    ///
    /// # Errors
    /// * `ApiError::Unauthorized` if the HTTP status is 401 Unauthorized.
    /// * `ApiError::HttpStatus` for other non-success HTTP statuses, with status and endpoint info.
    /// * `ApiError::HttpRequest` if the HTTP request fails.
    /// * `ApiError::Serialization` if the query cannot be serialized.
    /// * `ApiError::JsonParseDetailed` if the response body cannot be parsed into a `BundleItemsResponse`.
    pub async fn get_bundle(
        &self,
        blog_name: &str,
        bundle_id: &str,
        query: &BundleQuery,
    ) -> ResultApi<BundleItemsResponse> {
        let query_string = serde_urlencoded::to_string(query).map_err(ApiError::Serialization)?;

        // query_string is already urlencoded by serde_urlencoded — do not re-encode.
        let path = format!(
            "blog/{}/bundle/{}/post/?{query_string}",
            encode_segment(blog_name),
            encode_segment(bundle_id)
        );

        let response = self.get_request(&path).await?;
        let response = self.handle_response(&path, response).await?;

        self.parse_json(response).await
    }
}
