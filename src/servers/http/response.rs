use bytes::Bytes;
use hyper::{Body, Response, StatusCode};

use crate::worker::WorkerResponse;

pub(super) struct HttpResponse {
    pub data: Bytes,
}

impl From<Bytes> for HttpResponse {
    #[cfg_attr(feature = "coverage", inline(never))]
    #[cfg_attr(not(feature = "coverage"), inline(always))]
    fn from(data: Bytes) -> Self {
        Self { data }
    }
}

impl From<&'static str> for HttpResponse {
    #[cfg_attr(feature = "coverage", inline(never))]
    #[cfg_attr(not(feature = "coverage"), inline(always))]
    fn from(slice: &'static str) -> Self {
        Self::from(Bytes::from(slice.as_bytes()))
    }
}

impl From<Vec<u8>> for HttpResponse {
    #[cfg_attr(feature = "coverage", inline(never))]
    #[cfg_attr(not(feature = "coverage"), inline(always))]
    fn from(vec: Vec<u8>) -> Self {
        Self::from(Bytes::from(vec))
    }
}

impl From<String> for HttpResponse {
    #[cfg_attr(feature = "coverage", inline(never))]
    #[cfg_attr(not(feature = "coverage"), inline(always))]
    fn from(string: String) -> Self {
        Self::from(Bytes::from(string))
    }
}

impl From<WorkerResponse> for HttpResponse {
    fn from(response: WorkerResponse) -> Self {
        match response {
            WorkerResponse::Announce(_) => Self::from("Announce"),
            WorkerResponse::Scrape => Self::from("Scrape"),
            WorkerResponse::None => Self::from("None"),
        }
    }
}

impl Into<Response<Body>> for HttpResponse {
    fn into(self) -> Response<Body> {
        let mut response = Response::new(self.data.into());
        *response.status_mut() = StatusCode::OK;
        response
    }
}
