use std::fmt;

use serde::Deserialize;

/// The body BankID sends with a 4xx or 5xx response (`error_response`).
///
/// Error codes are kept as strings: BankID "may introduce new error codes without prior notice".
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ApiError {
    pub error_code: String,
    pub details: String,
}

impl std::error::Error for ApiError {}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "(errorCode: {}, details: {})",
            self.error_code, self.details
        )
    }
}

pub use reqwest::Error;
pub type Result<T, E = Error> = std::result::Result<T, E>;
