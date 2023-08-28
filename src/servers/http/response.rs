use std::{
    convert::Infallible,
    pin::Pin,
    task::{Context, Poll},
};

use bytes::Bytes;
use futures::Stream;
use http_body_util::{Either, Full, StreamBody};
use hyper::{body::Frame, Response, StatusCode};
use ts_utils::Shared;

pub(super) type Body = Either<Full<Bytes>, StreamBody<BodyStream>>;

/// The `HttpResponse` struct represents an HTTP response containing the response body.
#[derive(Debug)]
pub(super) struct HttpResponse {
    /// The body of the HTTP response represented as a `Body` object.
    pub body: Body,
}

impl From<Bytes> for HttpResponse {
    fn from(data: Bytes) -> Self {
        Self {
            body: Either::Left(Full::new(data)),
        }
    }
}

impl From<&'static str> for HttpResponse {
    fn from(slice: &'static str) -> Self {
        Self::from(Bytes::from(slice.as_bytes()))
    }
}

impl From<Vec<u8>> for HttpResponse {
    fn from(vec: Vec<u8>) -> Self {
        Self::from(Bytes::from(vec))
    }
}

impl From<String> for HttpResponse {
    fn from(string: String) -> Self {
        Self::from(Bytes::from(string))
    }
}

impl From<BodyStream> for HttpResponse {
    fn from(stream: BodyStream) -> Self {
        Self {
            body: Either::Right(stream.into()),
        }
    }
}

impl Into<Response<Body>> for HttpResponse {
    fn into(self) -> Response<Body> {
        let mut response = Response::new(self.body);
        *response.status_mut() = StatusCode::OK;
        response
    }
}

#[derive(Debug)]
pub(super) struct BodyStream {
    data: Data,
    buf_pos: usize,
}

#[allow(unused)]
impl BodyStream {
    fn new(data: Data) -> Self {
        Self { data, buf_pos: 0 }
    }
}

impl From<Bytes> for BodyStream {
    fn from(data: Bytes) -> Self {
        Self::new(Data::Owned(data))
    }
}

impl From<Shared<Bytes>> for BodyStream {
    fn from(data: Shared<Bytes>) -> Self {
        Self::new(Data::Shared(data))
    }
}

const CHUNK_SIZE: usize = 4096;

impl Stream for BodyStream {
    type Item = Result<Frame<Bytes>, Infallible>;

    fn poll_next(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.buf_pos >= self.data.len() {
            return Poll::Ready(None);
        }

        let remaining_data = &self.data[self.buf_pos..];
        let bytes_to_read = std::cmp::min(CHUNK_SIZE, remaining_data.len());
        let buf: Bytes = Bytes::copy_from_slice(&remaining_data[..bytes_to_read]);

        self.buf_pos += bytes_to_read;

        Poll::Ready(Some(Ok(Frame::data(buf))))
    }
}

impl Into<StreamBody<BodyStream>> for BodyStream {
    fn into(self) -> StreamBody<BodyStream> {
        StreamBody::new(self)
    }
}

#[derive(Debug)]
enum Data {
    Owned(Bytes),
    Shared(Shared<Bytes>),
}

impl std::ops::Deref for Data {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        match self {
            Data::Owned(bytes) => bytes,
            Data::Shared(bytes) => bytes,
        }
    }
}
