use bytes::Bytes;
use hyper::{Body, Response, StatusCode};

pub(super) struct HttpResponse {
    pub data: Bytes,
}

impl From<Bytes> for HttpResponse {
    #[inline]
    fn from(data: Bytes) -> Self {
        Self { data }
    }
}

impl From<&'static str> for HttpResponse {
    #[inline]
    fn from(slice: &'static str) -> Self {
        Self::from(Bytes::from(slice.as_bytes()))
    }
}

impl From<Vec<u8>> for HttpResponse {
    #[inline]
    fn from(vec: Vec<u8>) -> Self {
        Self::from(Bytes::from(vec))
    }
}

impl Into<Response<Body>> for HttpResponse {
    #[inline]
    fn into(self) -> Response<Body> {
        let mut response = Response::new(self.data.into());
        *response.status_mut() = StatusCode::OK;
        response
    }
}
