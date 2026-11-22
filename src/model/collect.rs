use serde::{Deserialize, Serialize};

/// The response to `/collect` (`collect_response`), by `status`.
///
/// Hint codes are kept as strings: BankID "may introduce new hint codes without prior notice" and requires that
/// unknown ones are handled.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum CollectResponse {
    /// The order is being processed. Collect again in two seconds.
    #[serde(rename_all = "camelCase")]
    Pending {
        /// The orderRef in question.
        order_ref: String,
        /// Describes the status of the order.
        hint_code: String,
    },
    /// The order failed. This is final: the same orderRef must not be collected again.
    #[serde(rename_all = "camelCase")]
    Failed {
        /// The orderRef in question.
        order_ref: String,
        /// Describes the status of the order.
        hint_code: String,
    },
    #[serde(rename_all = "camelCase")]
    Complete {
        /// The orderRef in question.
        order_ref: String,
        /// Only present for complete orders.
        completion_data: CompletionData,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CollectPayload {
    /// Used to collect the status of the order.
    pub order_ref: String,
}

/// The user and the completed order (`collect_response_completionData`).
///
/// BankID: "You should retain completion data for future reference, compliance and audit purposes." The signature
/// and OCSP response identify the user as fully as `user` does, so `Debug` prints none of the three.
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CompletionData {
    /// Information related to the user.
    pub user: User,
    /// Information related to the device.
    pub device: Device,
    /// Which additional identity confirmations were part of the order.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step_up: Option<StepUpCompletion>,
    /// The date the BankID was issued, as `YYYY-MM-DD` with a `Z` suffix.
    pub bank_id_issue_date: String,
    /// The signature: a base 64 encoded XML signature.
    pub signature: String,
    /// The base 64 encoded OCSP response.
    pub ocsp_response: String,
    /// Present only when the order asked for it with `returnRisk`, and absent when the risk could not be
    /// calculated.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub risk: Option<Risk>,
}

impl std::fmt::Debug for CompletionData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompletionData")
            .field("user", &self.user)
            .field("device", &self.device)
            .field("step_up", &self.step_up)
            .field("bank_id_issue_date", &self.bank_id_issue_date)
            .field(
                "signature",
                &format_args!("<{} bytes>", self.signature.len()),
            )
            .field(
                "ocsp_response",
                &format_args!("<{} bytes>", self.ocsp_response.len()),
            )
            .field("risk", &self.risk)
            .finish()
    }
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct User {
    /// The user's Swedish national identification number, 12 digits.
    pub personal_number: String,
    /// The given name and surname of the user
    pub name: String,
    /// The given name of the user.
    pub given_name: String,
    /// The surname of the user.
    pub surname: String,
}

impl std::fmt::Debug for User {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("User").finish_non_exhaustive()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Device {
    /// The IP address of the user agent as the BankID server sees it. For an order started with the autostart token,
    /// BankID suggests checking that it matches the address the relying party observes.
    pub ip_address: String,
    /// The unique hardware identifier of the user's device.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uhi: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct StepUpCompletion {
    /// Whether an MRTD check was performed before the order was completed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mrtd: Option<bool>,
    /// Whether the order was stepped up with facial recognition.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub face: Option<bool>,
}

/// BankID's risk indication for a completed order.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub enum Risk {
    /// "No or low risk identified in the available order data."
    Low,
    /// "Might require further action, investigation or follow-up by you based on the order data." The OpenAPI
    /// document names this value `moderate`; BankID's risk indication guide calls the level "medium", which is
    /// accepted too.
    #[serde(alias = "medium")]
    Moderate,
    /// "The order should be blocked or cancelled by you and needs further action, investigation or follow-up."
    High,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_documented_complete_example() {
        let response: CollectResponse = serde_json::from_value(serde_json::json!({
            "orderRef": "131daac9-16c6-4618-beb0-365768f37288",
            "status": "complete",
            "completionData": {
                "user": {
                    "personalNumber": "200001012384",
                    "name": "Alex Johnson",
                    "givenName": "Alex",
                    "surname": "Johnson"
                },
                "device": {
                    "ipAddress": "192.168.1.1",
                    "uhi": "OZvYM9VvyiAmG7NA5jU5zqGcVpo="
                },
                "bankIdIssueDate": "2023-04-01Z",
                "stepUp": { "mrtd": true },
                "signature": "PHNpZ25hdHVyZT4=",
                "ocspResponse": "MIIB",
                "risk": "low",
                "someFutureField": { "ignored": true }
            }
        }))
        .unwrap();

        let CollectResponse::Complete {
            order_ref,
            completion_data,
        } = response
        else {
            panic!("expected a complete order");
        };
        assert_eq!(order_ref, "131daac9-16c6-4618-beb0-365768f37288");
        assert_eq!(completion_data.user.personal_number, "200001012384");
        assert_eq!(completion_data.device.ip_address, "192.168.1.1");
        assert_eq!(
            completion_data.step_up,
            Some(StepUpCompletion {
                mrtd: Some(true),
                face: None
            })
        );
        assert_eq!(completion_data.risk, Some(Risk::Low));

        let debug = format!("{completion_data:?}");
        assert!(!debug.contains("200001012384"));
        assert!(!debug.contains("Alex"));
        assert!(!debug.contains("PHNpZ25hdHVyZT4="));
    }

    #[test]
    fn parses_pending_and_failed() {
        let pending: CollectResponse = serde_json::from_value(serde_json::json!({
            "orderRef": "131daac9-16c6-4618-beb0-365768f37288",
            "status": "pending",
            "hintCode": "userSign"
        }))
        .unwrap();
        assert_eq!(
            pending,
            CollectResponse::Pending {
                order_ref: "131daac9-16c6-4618-beb0-365768f37288".into(),
                hint_code: "userSign".into()
            }
        );

        let failed: CollectResponse = serde_json::from_value(serde_json::json!({
            "orderRef": "131daac9-16c6-4618-beb0-365768f37288",
            "status": "failed",
            "hintCode": "someHintCodeBankIdAddsLater"
        }))
        .unwrap();
        assert_eq!(
            failed,
            CollectResponse::Failed {
                order_ref: "131daac9-16c6-4618-beb0-365768f37288".into(),
                hint_code: "someHintCodeBankIdAddsLater".into()
            }
        );
    }

    #[test]
    fn risk_levels() {
        for (json, risk) in [
            ("low", Risk::Low),
            ("moderate", Risk::Moderate),
            ("medium", Risk::Moderate),
            ("high", Risk::High),
        ] {
            assert_eq!(serde_json::from_value::<Risk>(json.into()).unwrap(), risk);
        }
        assert!(serde_json::from_value::<Risk>("unheard-of".into()).is_err());
    }
}
