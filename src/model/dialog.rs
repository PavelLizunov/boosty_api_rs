use serde::Deserialize;

use crate::{
    media_content::{self, ContentItem},
    model::{CurrencyPrices, MediaData},
    traits::HasContent,
};

/// API response containing a page of dialogs (direct-message threads).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DialogsResponse {
    /// Dialogs on this page.
    pub data: Vec<Dialog>,
    /// Pagination info.
    pub extra: DialogsExtra,
}

/// Dialogs pagination info.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DialogsExtra {
    /// Offset for the next page.
    pub offset: u64,
    /// Total number of dialogs.
    pub total: u64,
}

/// A direct-message thread between the current user and another user.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Dialog {
    /// Unique dialog identifier.
    pub id: u64,
    /// Creation timestamp (unix epoch).
    pub created_at: i64,
    /// The other participant of the dialog.
    pub chatmate: Chatmate,
    /// Most recent message, if the dialog is not empty.
    pub last_message: Option<Message>,
    /// Number of unread messages in this dialog.
    pub unread_msg_count: u32,
}

/// The other participant in a dialog.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Chatmate {
    /// Unique user identifier.
    pub id: u64,
    /// Blog URL / slug of the user.
    pub url: String,
    /// Display name.
    pub name: String,
    /// Preferred currency (e.g. "RUB").
    pub currency: Option<String>,
    /// Blog's base currency.
    pub blog_currency: Option<String>,
    /// Currencies the user's blog accepts.
    pub accepted_currencies: Option<Vec<String>>,
    /// Whether this is an official account.
    pub is_official: bool,
    /// Whether the user has an avatar.
    pub has_avatar: bool,
    /// URL of the avatar.
    pub avatar_url: String,
    /// Whether the user's blog is marked as adult content.
    pub has_adult_content: Option<bool>,
    /// Whether the user is a blogger (has their own blog).
    pub is_blogger: Option<bool>,
}

/// API response containing a page of messages within a dialog.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessagesResponse {
    /// Messages on this page.
    pub data: Vec<Message>,
    /// Pagination info.
    pub extra: MessagesExtra,
}

/// Messages pagination info.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessagesExtra {
    /// Whether this is the last page.
    pub is_last: bool,
    /// Offset (last message id) to request the next page.
    pub offset: u64,
}

/// A single direct message.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    /// Unique message identifier.
    pub id: u64,
    /// Dialog this message belongs to.
    pub dialog_id: u64,
    /// Creation timestamp (unix epoch).
    pub created_at: i64,
    /// Author user id.
    pub author_id: u64,
    /// Whether the message has been read.
    pub is_read: bool,
    /// Whether the message is paid.
    pub is_paid: bool,
    /// Whether the message is deleted.
    pub is_deleted: bool,
    /// Whether the platform fee has been paid.
    pub is_fee_paid: bool,
    /// Price to unlock the message (fractional values occur on the live API).
    pub price: f64,
    /// Price details in various currencies.
    pub currency_prices: CurrencyPrices,
    /// Message content blocks (text, links, media).
    pub data: Vec<MediaData>,
    /// Teaser content shown before unlocking a paid message.
    pub teaser: Vec<MediaData>,
    /// Attachment counters by media type.
    pub attachments: MessageAttachments,
    /// Kind of preview (e.g. "text", "image").
    pub preview_type: String,
    /// Whether the message is behind a paywall (absent in some contexts).
    pub pay_wall: Option<bool>,
    /// Donation payload attached to the message, if any (unstructured).
    pub donation: Option<serde_json::Value>,
}

impl HasContent for Message {
    /// Extracts media items from the message into a vector of `ContentItem`.
    fn extract_content(&self) -> Vec<ContentItem> {
        media_content::extract_content(&self.data)
    }
}

/// Attachment counters for a message, grouped by media type.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageAttachments {
    /// Text block count.
    pub text: AttachmentCount,
    /// File attachment count.
    pub files: AttachmentCount,
    /// Audio attachment count.
    pub audios: AttachmentCount,
    /// Image attachments (count + preview).
    pub images: MediaAttachmentCount,
    /// Video attachments (count + preview).
    pub videos: MediaAttachmentCount,
}

/// A simple attachment counter.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentCount {
    /// Number of attachments of this type.
    pub count: u32,
}

/// An attachment counter with a preview URL (images / videos).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaAttachmentCount {
    /// Number of attachments of this type.
    pub count: u32,
    /// Preview image URL (may be empty).
    pub preview_url: String,
}
