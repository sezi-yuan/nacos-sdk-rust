use std::{time::Duration, any::Any};

use reqwest::{header::{HeaderMap, HeaderValue}, StatusCode, Method};
use serde::{Serialize, de::DeserializeOwned};

use crate::error::*;

#[derive(Clone)]
pub struct HttpClient {
    inner: reqwest::Client,
}

impl HttpClient {
    pub fn new() -> HttpClient {
        let http_client = reqwest::ClientBuilder::new()
            .connect_timeout(Duration::from_secs(6))
            .tcp_keepalive(Some(Duration::from_secs(10)))
            .default_headers(Self::default_headers())
            .pool_max_idle_per_host(3)
            .pool_idle_timeout(Duration::from_secs(30))
            .build()
            .expect("failed to build http client");
        
            HttpClient {
            inner: http_client
        }
    }

    fn default_headers() -> HeaderMap {
        let mut headers = HeaderMap::new();
        let client_version = "nacos-sdk-rust:".to_string() + std::env!("CARGO_PKG_VERSION_MAJOR");
        headers.insert("Client-Version", HeaderValue::try_from(client_version).expect("[default_header]never happen"));
        headers.insert("User-Agent", HeaderValue::from_static("reqwest-0-11"));
        headers.insert("Accept-Encoding", HeaderValue::from_static("gzip,deflate,sdch"));
        headers.insert("Requester", HeaderValue::from_static("Keep-Alive"));
        headers.insert("Request-Module", HeaderValue::from_static("naming"));

        headers
    }

    pub async fn request_json<T: DeserializeOwned + Any, Req: Serialize + ?Sized>(
        &self, base: &[String], path: &str, method: reqwest::Method, data: &Req
    ) -> Result<T> {
        let resp_text = self.request_str(base, path, method, data).await?;
        Ok(serde_json::from_str(resp_text.as_str())?)
    }

    pub async fn request_str<Req: Serialize + ?Sized>(
        &self, base: &[String], path: &str, method: reqwest::Method, data: &Req
    ) -> Result<String> {
        let mut index = rand::random::<usize>() % base.len();

        for (i, _) in base.iter().enumerate() {
            let url = format!("{}{}", &base[index], path);
            let res = self.send_request(url.as_str(), method.clone(), data).await;
            match res {
                Ok(resp) => return Ok(resp),
                Err(error) => {
                    log::error!("call nacos server[{}] error: {}", url, error);
                }
            }
            index = (index + i) % base.len()
        }
        Err(Error::Custom(format!("retry {} times http request failed", base.len())))
    }

    async fn send_request<Req: Serialize + ?Sized>(
        &self, url: &str, method: reqwest::Method, data: &Req
    ) -> Result<String> {
        log::trace!("send http request: {}", url);
        let request = self.inner.request(method.clone(), url);
        let result = match method {
            Method::GET => request.query(data),
            _ => request.form(data)
        }
        .header("RequestId", uuid::Uuid::new_v4().to_string())
        .send().await?;

        
        match result.status() {
            StatusCode::OK => {
                let resp_text = result.text().await?;
                log::debug!("[request_nacos]path: {} resp: {:?}", url, resp_text);
                Ok(resp_text)
            },
            code@_ => Err(Error::NacosRemote(code, result.text().await?))
        }
    }
}