use serde::{Deserialize, Serialize};

use crate::model::{App, OrderResponse, Requirement, UserVisibleDataFormat, Web};

pub type AuthenticateResponse = OrderResponse;

/// The body of `/auth` (`auth_request`).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct AuthenticatePayload {
    /// The user IP address as seen by RP. String. IPv4 and IPv6 is allowed.
    /// Note the importance of using the correct IP address. It must be the IP address
    /// representing the user agent (the end user device) as seen by the RP. If there is a
    /// proxy for inbound traffic, special considerations may need to be taken to get the
    /// correct address. In some use cases the IP address is not available, for instance for
    /// voice based services. In this case, the internal representation of those systems IP
    /// address is ok to use
    pub end_user_ip: String,
    /// Requirements on how the auth or sign order must be performed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requirement: Option<Requirement>,
    /// Text shown to the user during the order, UTF-8 then base 64 encoded; at most 1 500 characters encoded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_visible_data: Option<String>,
    /// Data not shown to the user, base 64 encoded; at most 1 500 characters encoded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_non_visible_data: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_visible_data_format: Option<UserVisibleDataFormat>,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_only_what_is_set() {
        let payload = AuthenticatePayload {
            end_user_ip: "192.168.1.2".into(),
            return_risk: Some(true),
            web: Some(Web {
                referring_domain: Some("example.com".into()),
                ..Default::default()
            }),
            ..Default::default()
        };
        assert_eq!(
            serde_json::to_value(&payload).unwrap(),
            serde_json::json!({
                "endUserIp": "192.168.1.2",
                "returnRisk": true,
                "web": { "referringDomain": "example.com" },
            })
        );
    }
}
