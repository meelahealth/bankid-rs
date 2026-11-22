//! A local stand-in for the BankID RP API: HTTPS with a client certificate required, as BankID's servers are, on
//! certificates generated for the test run.

use std::{
    convert::Infallible,
    net::SocketAddr,
    sync::{Arc, Mutex},
};

use bankid::config::{Config, Identity, Url};
use http_body_util::{BodyExt, Full};
use hyper::{body::Bytes, service::service_fn, Response, StatusCode};
use hyper_util::rt::TokioIo;
use rcgen::{
    BasicConstraints, CertificateParams, CertifiedIssuer, ExtendedKeyUsagePurpose, IsCa, KeyPair,
};
use tokio::net::TcpListener;
use tokio_rustls::{
    rustls::{
        self,
        pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer},
    },
    TlsAcceptor,
};

pub struct Pki {
    pub server_ca_pem: String,
    pub client_identity_pem: String,
    server_cert: CertificateDer<'static>,
    server_key: PrivatePkcs8KeyDer<'static>,
    client_ca: CertificateDer<'static>,
}

fn ca() -> CertifiedIssuer<'static, KeyPair> {
    let mut params = CertificateParams::new(Vec::<String>::new()).unwrap();
    params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    CertifiedIssuer::self_signed(params, KeyPair::generate().unwrap()).unwrap()
}

impl Pki {
    pub fn generate() -> Self {
        let server_ca = ca();
        let server_key = KeyPair::generate().unwrap();
        let mut server_params = CertificateParams::new(vec!["127.0.0.1".to_string()]).unwrap();
        server_params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];
        let server_cert = server_params.signed_by(&server_key, &server_ca).unwrap();

        let client_ca = ca();
        let client_key = KeyPair::generate().unwrap();
        let mut client_params = CertificateParams::new(Vec::<String>::new()).unwrap();
        client_params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ClientAuth];
        let client_cert = client_params.signed_by(&client_key, &client_ca).unwrap();

        Self {
            server_ca_pem: server_ca.pem(),
            client_identity_pem: format!("{}{}", client_cert.pem(), client_key.serialize_pem()),
            server_cert: server_cert.der().clone(),
            server_key: PrivatePkcs8KeyDer::from(server_key.serialize_der()),
            client_ca: client_ca.der().clone(),
        }
    }

    pub fn identity(&self) -> Identity {
        Identity::from_pem(self.client_identity_pem.as_bytes()).unwrap()
    }

    fn server_config(&self) -> rustls::ServerConfig {
        let provider = Arc::new(rustls::crypto::ring::default_provider());
        let mut roots = rustls::RootCertStore::empty();
        roots.add(self.client_ca.clone()).unwrap();
        let verifier = rustls::server::WebPkiClientVerifier::builder_with_provider(
            Arc::new(roots),
            provider.clone(),
        )
        .build()
        .unwrap();
        rustls::ServerConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()
            .unwrap()
            .with_client_cert_verifier(verifier)
            .with_single_cert(
                vec![self.server_cert.clone()],
                PrivateKeyDer::Pkcs8(self.server_key.clone_key()),
            )
            .unwrap()
    }
}

pub struct Reply {
    status: StatusCode,
    headers: Vec<(&'static str, String)>,
    body: String,
}

impl Reply {
    pub fn json(value: serde_json::Value) -> Self {
        Self::status(StatusCode::OK, value)
    }

    pub fn status(status: StatusCode, value: serde_json::Value) -> Self {
        Self {
            status,
            headers: vec![("content-type", "application/json".to_string())],
            body: value.to_string(),
        }
    }

    pub fn with_header(mut self, name: &'static str, value: &str) -> Self {
        self.headers.push((name, value.to_string()));
        self
    }
}

type Handler = Arc<dyn Fn(&str, &serde_json::Value) -> Reply + Send + Sync>;

pub struct FakeRpApi {
    pub pki: Pki,
    addr: SocketAddr,
    requests: Arc<Mutex<Vec<(String, serde_json::Value)>>>,
}

impl FakeRpApi {
    /// Serves every request with `handler`, given the request path below `/rp/v6.0/` and its JSON body.
    pub async fn start(
        handler: impl Fn(&str, &serde_json::Value) -> Reply + Send + Sync + 'static,
    ) -> Self {
        let pki = Pki::generate();
        let acceptor = TlsAcceptor::from(Arc::new(pki.server_config()));
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let handler: Handler = Arc::new(handler);

        let log = requests.clone();
        tokio::spawn(async move {
            loop {
                let Ok((tcp, _)) = listener.accept().await else {
                    return;
                };
                let acceptor = acceptor.clone();
                let handler = handler.clone();
                let log = log.clone();
                tokio::spawn(async move {
                    let Ok(tls) = acceptor.accept(tcp).await else {
                        return;
                    };
                    let service =
                        service_fn(move |request: hyper::Request<hyper::body::Incoming>| {
                            let handler = handler.clone();
                            let log = log.clone();
                            async move {
                                let path = request
                                    .uri()
                                    .path()
                                    .strip_prefix("/rp/v6.0/")
                                    .unwrap_or(request.uri().path())
                                    .to_string();
                                let body = request.into_body().collect().await.unwrap().to_bytes();
                                let body: serde_json::Value = serde_json::from_slice(&body)
                                    .unwrap_or(serde_json::Value::Null);
                                log.lock().unwrap().push((path.clone(), body.clone()));
                                let reply = handler(&path, &body);
                                let mut response = Response::builder().status(reply.status);
                                for (name, value) in reply.headers {
                                    response = response.header(name, value);
                                }
                                Ok::<_, Infallible>(
                                    response.body(Full::new(Bytes::from(reply.body))).unwrap(),
                                )
                            }
                        });
                    let _ = hyper::server::conn::http1::Builder::new()
                        .serve_connection(TokioIo::new(tls), service)
                        .await;
                });
            }
        });

        Self {
            pki,
            addr,
            requests,
        }
    }

    pub fn url(&self) -> Url {
        Url::parse(&format!("https://{}/rp/v6.0", self.addr)).unwrap()
    }

    pub fn config(&self) -> Config {
        Config::new(
            self.url(),
            self.pki.server_ca_pem.clone(),
            self.pki.identity(),
        )
    }

    pub fn requests(&self) -> Vec<(String, serde_json::Value)> {
        self.requests.lock().unwrap().clone()
    }
}
