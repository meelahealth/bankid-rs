//! BankID configuration objects.
//!
//! Used to populate [`Config`] that is ultimately used to construct a [`BankID`][crate::client::BankID] client.
//!
//! The endpoints and trust anchors below are the ones BankID publishes at
//! <https://developers.bankid.com/getting-started/environments> (read 2026-11-22). Any other endpoint, such as a
//! local fake of the RP API, is reached by giving [`Config::new`] its URL and the CA that issued its certificate.

use std::time::Duration;

pub use reqwest::header::HeaderName;
pub use reqwest::Identity;
pub use url::Url;

/// Everything the client needs to reach one RP API endpoint.
///
/// `ca` is the PEM encoding of the certificates that are trusted to issue the server's TLS certificate. They are
/// the only trust anchors the client uses: the system and web PKI roots are not consulted, as BankID directs ("You
/// must use the issuer of the certificate as trusted root").
#[derive(Debug, Clone)]
pub struct Config {
    /// The relying party's client certificate and private key.
    pub identity: Identity,
    /// The RP API base URL, up to and including the version segment, e.g. `https://appapi2.bankid.com/rp/v6.0`.
    pub url: Url,
    /// One or more PEM-encoded CA certificates.
    pub ca: String,
    /// A timeout applied to each request as a whole. `None` leaves requests without one.
    pub timeout: Option<Duration>,
    /// A response carrying any of these headers is treated as failed, whatever its status, and nothing from it is
    /// used. This lets a caller refuse a response that marks itself as coming from something other than BankID.
    pub refused_response_headers: Vec<HeaderName>,
}

impl Config {
    pub fn new(url: Url, ca: impl Into<String>, identity: Identity) -> Self {
        Self {
            identity,
            url,
            ca: ca.into(),
            timeout: None,
            refused_response_headers: Vec::new(),
        }
    }

    /// BankID's production environment.
    pub fn prod(identity: Identity) -> Self {
        Self::new(published_url(API_URL_PROD), CA_PROD, identity)
    }

    /// BankID's test environment.
    pub fn test(identity: Identity) -> Self {
        Self::new(published_url(API_URL_TEST), CA_TEST, identity)
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    pub fn refuse_response_header(mut self, name: HeaderName) -> Self {
        self.refused_response_headers.push(name);
        self
    }
}

fn published_url(url: &str) -> Url {
    Url::parse(url).expect("the published BankID API URLs are valid URLs")
}

pub const API_URL_TEST: &str = "https://appapi2.test.bankid.com/rp/v6.0";
pub const API_URL_PROD: &str = "https://appapi2.bankid.com/rp/v6.0";

/// `CN = Test BankID SSL Root CA v1 Test, OU = Infrastructure CA, O = Finansiell ID-Teknik BID AB`.
pub const CA_TEST: &str = include_str!("../../resources/test.ca");
/// `CN = BankID SSL Root CA v1, OU = Infrastructure CA, O = Finansiell ID-Teknik BID AB`.
pub const CA_PROD: &str = include_str!("../../resources/production.ca");
/// A test-environment relying-party certificate, as PKCS#12. The rustls backend cannot load PKCS#12: convert it to
/// PEM for [`Identity::from_pem`].
pub const P12_TEST: &[u8] = include_bytes!("../../resources/testcert.p12");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn published_urls_parse() {
        assert_eq!(
            published_url(API_URL_PROD).host_str(),
            Some("appapi2.bankid.com")
        );
        assert_eq!(
            published_url(API_URL_TEST).host_str(),
            Some("appapi2.test.bankid.com")
        );
    }

    #[test]
    fn bundled_cas_are_single_certificates() {
        for ca in [CA_PROD, CA_TEST] {
            let pems = pem::parse_many(ca).unwrap();
            assert_eq!(pems.len(), 1);
            assert_eq!(pems[0].tag(), "CERTIFICATE");
        }
    }
}
