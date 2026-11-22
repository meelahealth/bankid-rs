use serde::{Deserialize, Serialize};

pub use crate::model::UserVisibleDataFormat;
use crate::model::{App, OrderResponse, Requirement, Web};

pub type SignResponse = OrderResponse;

/// The body of `/sign` (`sign_request`).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct SignPayload {
    /// The user IP address as seen by RP. String. IPv4 and IPv6 is allowed.
    /// Note the importance of using the correct IP address. It must be the IP address
    /// representing the user agent (the end user device) as seen by the RP. If there is a
    /// proxy for inbound traffic, special considerations may need to be taken to get the
    /// correct address. In some use cases the IP address is not available, for instance for
    /// voice based services. In this case, the internal representation of those systems IP
    /// address is ok to use
    pub end_user_ip: String,
    /// The text to be displayed and signed. String. The text can be formatted using CR,
    /// LF and CRLF for new lines. The text must be encoded as UTF-8 and then base 64
    /// encoded. 1--40 000 characters after base 64 encoding.
    pub user_visible_data: String,
    /// Data not displayed to the user. String. The value must be base 64-encoded. 1-200
    /// 000 characters after base 64-encoding.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_non_visible_data: Option<String>,
    /// If present, this parameter indicates how userVisibleData is formatted: `plaintext`, or
    /// `simpleMarkdownV1`, for which see BankID's guide to formatted text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_visible_data_format: Option<UserVisibleDataFormat>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requirement: Option<Requirement>,
    /// Called by the BankID app when an order started with the autostart token completes. BankID asks that it
    /// carries a nonce for the session.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub return_url: Option<String>,
    /// Asks for the completion data to carry a [`Risk`][crate::model::Risk]. It relies on `end_user_ip` being
    /// correct.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub return_risk: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app: Option<App>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub web: Option<Web>,
}
