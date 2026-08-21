use reqwest::StatusCode;
use thiserror::Error;

/// Error during authentication with the Boosty API (e.g., token refresh).
#[derive(Error, Debug)]
pub enum AuthError {
    #[error("Invalid token format")]
    InvalidTokenFormat,

    #[error("Missing credentials: neither static access token nor refresh token + device_id set")]
    MissingCredentials,

    #[error("Empty access token")]
    EmptyAccessToken,

    #[error("Empty refresh token")]
    EmptyRefreshToken,

    #[error("Empty device_id")]
    EmptyDeviceId,

    #[error("HTTP request error during token refresh: {0}")]
    HttpRequest(#[from] reqwest::Error),

    #[error("Unexpected HTTP status {status} during token refresh, body: {body}")]
    HttpStatus { status: StatusCode, body: String },

    #[error("Failed to parse JSON response during token refresh: {0}")]
    ParseError(#[from] serde_json::Error),

    /// The callback registered to durably store the rotated refresh token
    /// failed. Carries only the `std::io::ErrorKind` — never the token or
    /// the raw I/O message, which may contain filesystem paths.
    #[error("Failed to persist rotated refresh token ({0:?})")]
    TokenPersist(std::io::ErrorKind),
}

/// Error when calling Boosty API endpoints (includes AuthError).
#[derive(Error, Debug)]
pub enum ApiError {
    #[error("Authentication error: {0}")]
    Auth(#[from] AuthError),

    #[error("HTTP request error when calling API: {0}")]
    HttpRequest(#[from] reqwest::Error),

    #[error("Unexpected HTTP status {status} when calling endpoint '{endpoint}'")]
    HttpStatus {
        status: StatusCode,
        endpoint: String,
    },

    #[error("Failed to parse response JSON: {error}")]
    JsonParseDetailed { error: String },

    #[error("Unauthorized (401): invalid or missing token")]
    Unauthorized,

    #[error("Failed to deserialize JSON into target type: {0}")]
    Deserialization(#[from] serde_json::Error),

    #[error("Failed to serialize JSON: {0}")]
    Serialization(#[from] serde_urlencoded::ser::Error),

    /// A collection endpoint could not prove that pagination completed.
    /// Labels are static so this error never carries upstream PII or IDs.
    #[error("Incomplete pagination for {resource}: {reason}")]
    Pagination {
        resource: &'static str,
        reason: &'static str,
    },

    #[error("Other error: {0}")]
    Other(String),
}

pub type ResultAuth<T> = Result<T, AuthError>;
pub type ResultApi<T> = Result<T, ApiError>;
