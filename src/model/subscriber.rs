use serde::Deserialize;
use std::collections::HashMap;

/// API response containing a paginated list of a blog's subscribers.
#[derive(Debug, Deserialize)]
pub struct SubscribersResponse {
    /// Subscribers on this page.
    pub data: Vec<Subscriber>,
    /// Total number of subscribers.
    pub total: u64,
    /// Page size.
    pub limit: u64,
    /// Offset of the current page.
    pub offset: u64,
}

/// A single subscriber of the current user's blog.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Subscriber {
    /// Unique user identifier.
    pub id: u64,
    /// Display name.
    pub name: String,
    /// Subscriber's email (may be empty).
    #[serde(default)]
    pub email: String,
    /// Whether the user has an avatar.
    pub has_avatar: bool,
    /// URL of the avatar.
    pub avatar_url: String,
    /// Whether this is an official account.
    pub is_official: bool,
    /// Whether the subscriber is blacklisted by the blog.
    pub is_black_listed: bool,
    /// Whether the platform fee is paid.
    pub is_fee_paid: bool,
    /// Whether the blogger can write to this subscriber.
    pub can_write: bool,
    /// Whether the user is currently subscribed.
    pub subscribed: bool,
    /// Subscription status (e.g. "active", "inactive").
    pub status: String,
    /// Subscription start timestamp (unix epoch).
    pub on_time: i64,
    /// Subscription end timestamp, if it has ended.
    pub off_time: Option<i64>,
    /// Next payment timestamp, if recurring.
    pub next_pay_time: Option<i64>,
    /// Current subscription price (fractional values occur on the live API).
    pub price: f64,
    /// Total amount this subscriber has paid.
    pub payments: f64,
    /// The subscription level the subscriber is on.
    pub level: SubscriberLevel,
}

impl Subscriber {
    /// Whether the subscription is currently active.
    ///
    /// Uses Boosty's own `status` field (`"active"` vs `"inactive"`), which is
    /// the authoritative signal a provisioning bridge should key on: active →
    /// the subscriber should have access; inactive → access should be paused.
    pub fn is_active(&self) -> bool {
        self.status.eq_ignore_ascii_case("active")
    }
}

/// The subscription level attached to a subscriber.
///
/// This is a lighter shape than [`crate::model::SubscriptionLevel`] — it has
/// no promos/external apps, but carries a nested `flags` object and optional
/// parent-level references.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscriberLevel {
    /// Unique identifier.
    pub id: u64,
    /// Level name.
    pub name: String,
    /// Base price (fractional values occur on the live API).
    pub price: f64,
    /// Price per currency (e.g. "RUB", "USD").
    pub currency_prices: HashMap<String, f64>,
    /// Creation timestamp (unix epoch).
    pub created_at: i64,
    /// Owner (blog) id.
    pub owner_id: u64,
    /// Whether the level is deleted.
    pub deleted: bool,
    /// Whether the level is hidden.
    pub is_hidden: bool,
    /// Whether the level has limited availability.
    pub is_limited: bool,
    /// Whether the level is archived.
    pub is_archived: bool,
    /// Nested flag object (mirrors the top-level flags).
    pub flags: LevelFlags,
    /// Content data blocks (unstructured).
    pub data: Vec<serde_json::Value>,
    /// Archival timestamp, if archived.
    pub archived_at: Option<i64>,
    /// Parent level id, if this level was upgraded from another.
    pub parent_id: Option<u64>,
    /// Parent level payload, if present (unstructured).
    pub parent: Option<serde_json::Value>,
}

/// Nested availability flags on a [`SubscriberLevel`].
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LevelFlags {
    /// Whether the level is hidden.
    pub is_hidden: bool,
    /// Whether the level has limited availability.
    pub is_limited: bool,
    /// Whether the level is archived.
    pub is_archived: bool,
}
