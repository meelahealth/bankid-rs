//! All BankID API endpoint request and response object
//!
//! The shapes follow BankID's Relying Party API v6.0 as published in its OpenAPI document
//! `openapi-auth-sign-6.1.11.yaml` (<https://developers.bankid.com/assets/openapi/openapi-auth-sign-6.1.11.yaml>,
//! read 2026-11-22). Responses are parsed leniently: BankID's general rules state that new return parameters may be
//! added without notice and that "the implementation shouldn't discard the complete response if it includes unknown
//! parameters".

use serde::{Deserialize, Serialize};

pub use authenticate::*;
pub use cancel::*;
pub use collect::*;
pub use sign::*;

pub mod authenticate;
pub mod cancel;
pub mod collect;
pub mod sign;

/// Requirements on how an auth or sign order must be performed (`auth_request_requirement` and
/// `sign_request_requirement`, which are identical in v6.0).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Requirement {
    /// Whether the order must be confirmed with a card reader. BankID advises always combining this with a
    /// `certificatePolicies` entry for a smart card.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub card_reader: Option<CardReader>,
    /// OIDs the user's certificate policies must match; one wildcard is allowed from position 5 (`1.2.752.78.*`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub certificate_policies: Vec<String>,
    /// Whether the user must confirm their identity with a valid Swedish passport or national ID card. Cannot be
    /// combined with `step_up`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mrtd: Option<bool>,
    /// The only personal identity number (12 digits) allowed to confirm the order.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub personal_number: Option<String>,
    /// Whether the user must confirm with their security code even if biometrics are activated.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pin_code: Option<bool>,
    /// Identity confirmations the user may choose between. Cannot be combined with `mrtd`, and `face` alone is not
    /// supported.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub step_up: Vec<StepUp>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CardReader {
    /// A card reader where the PIN code is entered on a computer keyboard, or a card reader of higher class.
    Class1,
    /// A card reader where the PIN code is entered on the reader.
    Class2,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum StepUp {
    /// Confirm identity with a valid Swedish passport or national ID card.
    Mrtd,
    /// Confirm identity with facial recognition.
    Face,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Copy)]
pub enum UserVisibleDataFormat {
    #[serde(rename = "plaintext")]
    Plaintext,
    #[serde(rename = "simpleMarkdownV1")]
    SimpleMarkdownV1,
}

/// Data about the relying party's app, sent when the order is started from it. Mutually exclusive with [`Web`].
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct App {
    /// The package name on Android, the bundle identifier on iOS.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app_identifier: Option<String>,
    #[serde(rename = "deviceOS", skip_serializing_if = "Option::is_none")]
    pub device_os: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_identifier: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_model_name: Option<String>,
}

/// Data about the relying party's web page, sent when the order is started from it. Mutually exclusive with [`App`].
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Web {
    /// BankID asks that this is not a session cookie, but a separate cookie or the hash of one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_identifier: Option<String>,
    /// The registrable domain that starts the BankID app, punycode-encoded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub referring_domain: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_agent: Option<String>,
}

/// The response to `/auth` and `/sign` (`order_response`).
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OrderResponse {
    /// Used to collect the status of the order, or to cancel it.
    pub order_ref: String,
    /// Used to compile the start URL.
    pub auto_start_token: String,
    /// Used to compute the animated QR code.
    pub qr_start_token: String,
    /// Used to compute the animated QR code. BankID: it "shouldn't be sent to your frontend".
    pub qr_start_secret: String,
}

impl std::fmt::Debug for OrderResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OrderResponse")
            .field("order_ref", &self.order_ref)
            .field("auto_start_token", &self.auto_start_token)
            .field("qr_start_token", &self.qr_start_token)
            .field("qr_start_secret", &"<redacted>")
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requirement_serializes_v6_fields() {
        let requirement = Requirement {
            card_reader: Some(CardReader::Class2),
            certificate_policies: vec!["1.2.752.78.1.2".into()],
            pin_code: Some(true),
            personal_number: Some("200001012384".into()),
            step_up: vec![StepUp::Mrtd, StepUp::Face],
            ..Default::default()
        };
        assert_eq!(
            serde_json::to_value(&requirement).unwrap(),
            serde_json::json!({
                "cardReader": "class2",
                "certificatePolicies": ["1.2.752.78.1.2"],
                "personalNumber": "200001012384",
                "pinCode": true,
                "stepUp": ["mrtd", "face"],
            })
        );
        assert_eq!(
            serde_json::to_value(Requirement::default()).unwrap(),
            serde_json::json!({})
        );
    }

    #[test]
    fn app_device_os_uses_the_documented_name() {
        let app = App {
            device_os: Some("IOS 16.7.7".into()),
            ..Default::default()
        };
        assert_eq!(
            serde_json::to_value(&app).unwrap(),
            serde_json::json!({ "deviceOS": "IOS 16.7.7" })
        );
    }

    #[test]
    fn order_response_parses_the_documented_example() {
        let response: OrderResponse = serde_json::from_value(serde_json::json!({
            "orderRef": "131daac9-16c6-4618-beb0-365768f37288",
            "autoStartToken": "7c40b5c9-fa74-49cf-b98c-bfe651f9a7c6",
            "qrStartToken": "67df3917-fa0d-44e5-b327-edcc928297f8",
            "qrStartSecret": "d28db9a7-4cde-429e-a983-359be676944c",
        }))
        .unwrap();
        assert_eq!(response.order_ref, "131daac9-16c6-4618-beb0-365768f37288");
        assert!(!format!("{response:?}").contains("d28db9a7"));
    }
}
