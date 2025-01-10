use std::convert::TryInto;
use std::sync::OnceLock;

use async_trait::async_trait;
use gleam_core::{Error, Result};
use http::{Request, Response};
use reqwest::{Certificate, Client};

static REQWEST_CLIENT: OnceLock<Client> = OnceLock::new();

#[derive(Debug)]
pub struct HttpClient;

impl HttpClient {
    pub fn new() -> Self {
        Self
    }

    pub fn boxed() -> Box<Self> {
        Box::new(Self::new())
    }
}

#[async_trait]
impl gleam_core::io::HttpClient for HttpClient {
    async fn send(&self, request: Request<Vec<u8>>) -> Result<Response<Vec<u8>>> {
        let request = request
            .try_into()
            .expect("Unable to convert HTTP request for use by reqwest library");
        let mut response = REQWEST_CLIENT
            .get_or_init(init_client)
            .execute(request)
            .await
            .map_err(Error::http)?;
        let mut builder = Response::builder()
            .status(response.status())
            .version(response.version());
        if let Some(headers) = builder.headers_mut() {
            std::mem::swap(headers, response.headers_mut());
        }
        builder
            .body(response.bytes().await.map_err(Error::http)?.to_vec())
            .map_err(Error::http)
    }
}

fn init_client() -> Client {
    match get_certificate() {
        Ok(cert) => Client::builder()
            .add_root_certificate(cert)
            .build()
            .expect("Unable to build reqwest client with certificate"),
        _ => Client::new(),
    }
}

fn get_certificate() -> Result<Certificate, Error> {
    let certificate_path = std::env::var("GLEAM_CACERTS_PATH")?;
    let certificate_bytes = std::fs::read(&certificate_path)?;

    match Certificate::from_pem(&certificate_bytes) {
        Ok(certificate) => Ok(certificate),
        Err(e) => Error::CannotReadCertificate {
            path: certificate_path,
        },
    }
}
