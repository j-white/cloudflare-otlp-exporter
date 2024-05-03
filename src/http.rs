use std::fmt::{Debug, Formatter};
use std::future::Future;
use opentelemetry_http::{Bytes, HttpClient, HttpError, Request, Response};
use worker::async_trait::async_trait;
use std::error::Error;

pub struct MyClient {
    inner: reqwest::Client,
}

impl MyClient {
    pub fn new() -> MyClient {
        let inner = reqwest::Client::new();
        Self { inner }
    }

    pub fn execute(
        &self,
        request: reqwest::Request,
    ) -> impl Future<Output = Result<reqwest::Response, reqwest::Error>> {
        self.inner.execute(request)
    }
}

impl Debug for MyClient {
    fn fmt(&self, _f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

#[async_trait]
impl HttpClient for MyClient {
    async fn send(&self, request: Request<Vec<u8>>) -> Result<Response<Bytes>, HttpError> {
        let res = Response::builder()
            .status(200)
            .body(Bytes::new())
            .unwrap();
        Ok(res)
    }


}
