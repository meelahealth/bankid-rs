use reqwest::{header::HeaderName, StatusCode};
use serde::{de::DeserializeOwned, Serialize};
use url::Url;

use crate::{
    config::Config,
    error::{ApiError, Result},
    model::{
        authenticate::{AuthenticatePayload, AuthenticateResponse},
        cancel::{CancelPayload, CancelResponse},
        collect::{CollectPayload, CollectResponse},
        sign::{SignPayload, SignResponse},
    },
};

/// Client for communicating with the BankID API.
///
/// The best way to instantiate the client is with an existing [`Config`]
/// using [`BankID::new`]
#[derive(Clone, Debug)]
pub struct BankID {
    pub(crate) client: reqwest::Client,
    base_url: Url,
    refused_response_headers: Vec<HeaderName>,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Error making HTTP request")]
    Http(#[from] reqwest::Error),

    #[error("BankID answered /{endpoint} with HTTP {status} {error}")]
    Api {
        endpoint: &'static str,
        status: StatusCode,
        error: ApiError,
    },

    #[error("BankID answered /{endpoint} with HTTP {status} and no error body")]
    Status {
        endpoint: &'static str,
        status: StatusCode,
    },

    #[error("Could not parse the /{endpoint} response")]
    InvalidJson {
        endpoint: &'static str,
        #[source]
        source: serde_json::Error,
    },

    #[error("The /{endpoint} response carried the refused header {header}")]
    RefusedResponse {
        endpoint: &'static str,
        header: HeaderName,
    },
}

impl Error {
    /// The error BankID reported, when it reported one.
    pub fn api_error(&self) -> Option<&ApiError> {
        match self {
            Error::Api { error, .. } => Some(error),
            _ => None,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum InitError {
    #[error("PEM error: {0}")]
    Pem(#[from] pem::PemError),

    #[error("The configured CA holds no certificate")]
    NoTrustAnchor,

    #[error("Reqwest error: {0}")]
    Reqwest(#[from] reqwest::Error),
}

impl BankID {
    /// Create and initialize a [`BankID`] using the given configuration.
    ///
    /// The client trusts only the CA certificates in [`Config::ca`] and follows no redirects.
    pub fn new(config: Config) -> Result<Self, InitError> {
        let Config {
            identity,
            url,
            ca,
            timeout,
            refused_response_headers,
        } = config;

        let mut builder = reqwest::Client::builder()
            .identity(identity)
            .tls_built_in_root_certs(false)
            .redirect(reqwest::redirect::Policy::none());

        let mut anchors = 0;
        for pem in pem::parse_many(&ca)? {
            if pem.tag() != "CERTIFICATE" {
                continue;
            }
            let certificate = reqwest::Certificate::from_der(pem.contents())?;
            builder = builder.add_root_certificate(certificate);
            anchors += 1;
        }
        if anchors == 0 {
            return Err(InitError::NoTrustAnchor);
        }

        if let Some(timeout) = timeout {
            builder = builder.timeout(timeout);
        }

        Ok(Self {
            client: builder.build()?,
            base_url: url,
            refused_response_headers,
        })
    }

    fn endpoint_url(&self, endpoint: &str) -> String {
        format!(
            "{}/{endpoint}",
            self.base_url.as_str().trim_end_matches('/')
        )
    }

    async fn send_payload<T: Serialize, U: DeserializeOwned>(
        &self,
        endpoint: &'static str,
        payload: &T,
    ) -> Result<U, Error> {
        let response = self
            .client
            .post(self.endpoint_url(endpoint))
            .json(payload)
            .send()
            .await?;

        if let Some(header) = self
            .refused_response_headers
            .iter()
            .find(|name| response.headers().contains_key(*name))
        {
            return Err(Error::RefusedResponse {
                endpoint,
                header: header.clone(),
            });
        }

        let status = response.status();
        let body = response.bytes().await?;

        if !status.is_success() {
            return Err(match serde_json::from_slice::<ApiError>(&body) {
                Ok(error) => Error::Api {
                    endpoint,
                    status,
                    error,
                },
                Err(_) => Error::Status { endpoint, status },
            });
        }

        serde_json::from_slice(&body).map_err(|source| Error::InvalidJson { endpoint, source })
    }

    /// Authenticate - Initiates an authentication order.
    ///
    /// Use the collect method to query the status of the order.
    /// If the request is successful, the orderRef and autoStartToken is returned.
    pub async fn authenticate(
        &self,
        payload: AuthenticatePayload,
    ) -> Result<AuthenticateResponse, Error> {
        self.send_payload("auth", &payload).await
    }

    /// Sign - Initiates an sign order.
    ///
    /// Use the collect method to query the status of the order.
    /// If the request is successful, the orderRef and autoStartToken is returned.
    pub async fn sign(&self, payload: SignPayload) -> Result<SignResponse, Error> {
        self.send_payload("sign", &payload).await
    }

    /// Collect - Collects the result of a sign or auth order using the orderRef as reference.
    ///
    /// RP should keep calling collect every two seconds as long as status indicates pending.
    /// RP must abort if status indicates failed. The user identity is returned when complete.
    pub async fn collect(&self, payload: CollectPayload) -> Result<CollectResponse, Error> {
        self.send_payload("collect", &payload).await
    }

    /// Cancel - Cancels an ongoing sign or auth order.
    ///
    /// This is typically used if the user cancels the order in your service or app.
    pub async fn cancel(&self, payload: CancelPayload) -> Result<CancelResponse, Error> {
        self.send_payload("cancel", &payload).await
    }
}
