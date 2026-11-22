mod support;

use bankid::{
    client::{BankID, Error, InitError},
    config::{Config, HeaderName},
    model::{AuthenticatePayload, CancelPayload, CollectPayload, CollectResponse, Risk},
};
use hyper::StatusCode;
use serde_json::json;
use support::{FakeRpApi, Pki, Reply};

const ORDER: &str = "131daac9-16c6-4618-beb0-365768f37288";

fn order_response() -> serde_json::Value {
    json!({
        "orderRef": ORDER,
        "autoStartToken": "7c40b5c9-fa74-49cf-b98c-bfe651f9a7c6",
        "qrStartToken": "67df3917-fa0d-44e5-b327-edcc928297f8",
        "qrStartSecret": "d28db9a7-4cde-429e-a983-359be676944c"
    })
}

#[tokio::test]
async fn authenticate_posts_to_the_configured_endpoint() {
    let api = FakeRpApi::start(|path, _| match path {
        "auth" => Reply::json(order_response()),
        _ => Reply::status(StatusCode::NOT_FOUND, json!({})),
    })
    .await;
    let client = BankID::new(api.config()).unwrap();

    let order = client
        .authenticate(AuthenticatePayload {
            end_user_ip: "192.168.1.2".into(),
            return_risk: Some(true),
            ..Default::default()
        })
        .await
        .unwrap();

    assert_eq!(order.order_ref, ORDER);
    assert_eq!(order.qr_start_token, "67df3917-fa0d-44e5-b327-edcc928297f8");
    assert_eq!(
        api.requests(),
        vec![(
            "auth".to_string(),
            json!({ "endUserIp": "192.168.1.2", "returnRisk": true })
        )]
    );
}

#[tokio::test]
async fn collect_and_cancel() {
    let api = FakeRpApi::start(|path, _| match path {
        "collect" => Reply::json(json!({
            "orderRef": ORDER,
            "status": "complete",
            "completionData": {
                "user": {
                    "personalNumber": "200001012384",
                    "name": "Alex Johnson",
                    "givenName": "Alex",
                    "surname": "Johnson"
                },
                "device": { "ipAddress": "192.168.1.1", "uhi": "OZvYM9VvyiAmG7NA5jU5zqGcVpo=" },
                "bankIdIssueDate": "2023-04-01Z",
                "signature": "PHNpZ25hdHVyZT4=",
                "ocspResponse": "MIIB",
                "risk": "high"
            }
        })),
        "cancel" => Reply::json(json!({})),
        _ => Reply::status(StatusCode::NOT_FOUND, json!({})),
    })
    .await;
    let client = BankID::new(api.config()).unwrap();

    let collected = client
        .collect(CollectPayload {
            order_ref: ORDER.into(),
        })
        .await
        .unwrap();
    let CollectResponse::Complete {
        completion_data, ..
    } = collected
    else {
        panic!("expected a complete order, got {collected:?}");
    };
    assert_eq!(completion_data.risk, Some(Risk::High));

    client
        .cancel(CancelPayload {
            order_ref: ORDER.into(),
        })
        .await
        .unwrap();
    assert_eq!(
        api.requests()[1],
        ("cancel".to_string(), json!({ "orderRef": ORDER }))
    );
}

#[tokio::test]
async fn api_errors_are_typed() {
    let api = FakeRpApi::start(|_, _| {
        Reply::status(
            StatusCode::BAD_REQUEST,
            json!({ "errorCode": "invalidParameters", "details": "No such order" }),
        )
    })
    .await;
    let client = BankID::new(api.config()).unwrap();

    let error = client
        .collect(CollectPayload {
            order_ref: ORDER.into(),
        })
        .await
        .unwrap_err();
    match &error {
        Error::Api {
            endpoint,
            status,
            error,
        } => {
            assert_eq!(*endpoint, "collect");
            assert_eq!(*status, StatusCode::BAD_REQUEST);
            assert_eq!(error.error_code, "invalidParameters");
            assert_eq!(error.details, "No such order");
        }
        other => panic!("expected an API error, got {other:?}"),
    }
    assert_eq!(error.api_error().unwrap().error_code, "invalidParameters");
}

#[tokio::test]
async fn a_status_without_an_error_body_is_reported() {
    let api =
        FakeRpApi::start(|_, _| Reply::status(StatusCode::SERVICE_UNAVAILABLE, json!(null))).await;
    let client = BankID::new(api.config()).unwrap();

    let error = client
        .cancel(CancelPayload {
            order_ref: ORDER.into(),
        })
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        Error::Status {
            endpoint: "cancel",
            status: StatusCode::SERVICE_UNAVAILABLE
        }
    ));
}

#[tokio::test]
async fn redirects_are_not_followed() {
    let api = FakeRpApi::start(|_, _| {
        Reply::status(StatusCode::FOUND, json!(null))
            .with_header("location", "https://example.com/rp/v6.0/auth")
    })
    .await;
    let client = BankID::new(api.config()).unwrap();

    let error = client
        .authenticate(AuthenticatePayload {
            end_user_ip: "192.168.1.2".into(),
            ..Default::default()
        })
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        Error::Status {
            status: StatusCode::FOUND,
            ..
        }
    ));
    assert_eq!(api.requests().len(), 1);
}

#[tokio::test]
async fn a_refused_header_fails_the_call_whatever_the_status() {
    let api =
        FakeRpApi::start(|_, _| Reply::json(order_response()).with_header("nb-mock", "bankid"))
            .await;
    let client = BankID::new(
        api.config()
            .refuse_response_header(HeaderName::from_static("nb-mock")),
    )
    .unwrap();

    let error = client
        .authenticate(AuthenticatePayload {
            end_user_ip: "192.168.1.2".into(),
            ..Default::default()
        })
        .await
        .unwrap_err();
    match error {
        Error::RefusedResponse { endpoint, header } => {
            assert_eq!(endpoint, "auth");
            assert_eq!(header.as_str(), "nb-mock");
        }
        other => panic!("expected a refused response, got {other:?}"),
    }
}

#[tokio::test]
async fn only_the_configured_ca_is_trusted() {
    let api = FakeRpApi::start(|_, _| Reply::json(order_response())).await;
    let other = Pki::generate();
    let client = BankID::new(Config::new(
        api.url(),
        other.server_ca_pem.clone(),
        api.pki.identity(),
    ))
    .unwrap();

    let error = client
        .authenticate(AuthenticatePayload {
            end_user_ip: "192.168.1.2".into(),
            ..Default::default()
        })
        .await
        .unwrap_err();
    assert!(matches!(error, Error::Http(_)));
    assert!(api.requests().is_empty());
}

#[tokio::test]
async fn the_server_requires_the_configured_client_identity() {
    let api = FakeRpApi::start(|_, _| Reply::json(order_response())).await;
    let other = Pki::generate();
    let client = BankID::new(Config::new(
        api.url(),
        api.pki.server_ca_pem.clone(),
        other.identity(),
    ))
    .unwrap();

    let error = client
        .authenticate(AuthenticatePayload {
            end_user_ip: "192.168.1.2".into(),
            ..Default::default()
        })
        .await
        .unwrap_err();
    assert!(matches!(error, Error::Http(_)));
    assert!(api.requests().is_empty());
}

#[test]
fn a_ca_without_a_certificate_is_refused() {
    let pki = Pki::generate();
    let config = Config::new(
        "https://127.0.0.1:1/rp/v6.0".parse().unwrap(),
        "-----BEGIN PRIVATE KEY-----\nAAAA\n-----END PRIVATE KEY-----\n",
        pki.identity(),
    );
    assert!(matches!(BankID::new(config), Err(InitError::NoTrustAnchor)));
}

#[test]
fn published_environments_build() {
    let pki = Pki::generate();
    BankID::new(Config::prod(pki.identity())).unwrap();
    BankID::new(Config::test(pki.identity())).unwrap();
}
