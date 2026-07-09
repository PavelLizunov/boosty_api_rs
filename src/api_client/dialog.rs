use crate::api_client::ApiClient;
use crate::error::ResultApi;
use crate::model::{DialogsResponse, Message, MessagesResponse};

impl ApiClient {
    /// Get a page of the current user's dialogs (direct-message threads).
    ///
    /// Requires an authenticated client (bearer token or refresh flow).
    ///
    /// # Arguments
    ///
    /// * `limit` - max dialogs to return (optional).
    /// * `offset` - pagination offset from a previous `DialogsExtra` (optional).
    ///
    /// # Errors
    ///
    /// - `ApiError::Unauthorized` if the HTTP status is 401 Unauthorized.
    /// - `ApiError::HttpStatus` for other non-success HTTP statuses.
    /// - `ApiError::HttpRequest` if the HTTP request fails.
    /// - `ApiError::JsonParseDetailed` if the body cannot be parsed into a `DialogsResponse`.
    pub async fn get_dialogs(
        &self,
        limit: Option<u32>,
        offset: Option<u64>,
    ) -> ResultApi<DialogsResponse> {
        let mut path = "dialog/".to_string();
        let mut params = Vec::new();
        if let Some(o) = offset {
            params.push(format!("offset={o}"));
        }
        if let Some(l) = limit {
            params.push(format!("limit={l}"));
        }
        if !params.is_empty() {
            path.push('?');
            path.push_str(&params.join("&"));
        }

        let response = self.get_request(&path).await?;
        let response = self.handle_response(&path, response).await?;

        self.parse_json(response).await
    }

    /// Get a page of messages within a dialog.
    ///
    /// # Arguments
    ///
    /// * `dialog_id` - the dialog identifier.
    /// * `limit` - max messages to return (optional).
    /// * `offset` - pagination offset from a previous `MessagesExtra` (optional).
    ///
    /// # Errors
    ///
    /// - `ApiError::Unauthorized` if the HTTP status is 401 Unauthorized.
    /// - `ApiError::HttpStatus` for other non-success HTTP statuses.
    /// - `ApiError::HttpRequest` if the HTTP request fails.
    /// - `ApiError::JsonParseDetailed` if the body cannot be parsed into a `MessagesResponse`.
    pub async fn get_dialog_messages(
        &self,
        dialog_id: u64,
        limit: Option<u32>,
        offset: Option<u64>,
    ) -> ResultApi<MessagesResponse> {
        let mut path = format!("dialog/{dialog_id}/message/");
        let mut params = Vec::new();
        if let Some(o) = offset {
            params.push(format!("offset={o}"));
        }
        if let Some(l) = limit {
            params.push(format!("limit={l}"));
        }
        if !params.is_empty() {
            path.push('?');
            path.push_str(&params.join("&"));
        }

        let response = self.get_request(&path).await?;
        let response = self.handle_response(&path, response).await?;

        self.parse_json(response).await
    }

    /// Get all messages in a dialog, following pagination until the last page.
    ///
    /// # Arguments
    ///
    /// * `dialog_id` - the dialog identifier.
    /// * `limit` - per-request page size (optional).
    ///
    /// # Returns
    ///
    /// All messages, oldest page first.
    ///
    /// # Errors
    ///
    /// Same as [`ApiClient::get_dialog_messages`].
    pub async fn get_all_dialog_messages(
        &self,
        dialog_id: u64,
        limit: Option<u32>,
    ) -> ResultApi<Vec<Message>> {
        let mut all_messages = Vec::new();
        let mut offset: Option<u64> = None;

        loop {
            let resp = self.get_dialog_messages(dialog_id, limit, offset).await?;

            if resp.data.is_empty() {
                break;
            }

            all_messages.extend(resp.data);

            if resp.extra.is_last {
                break;
            }

            offset = Some(resp.extra.offset);
        }

        Ok(all_messages)
    }
}
