use crate::{constants, worker};
use bytes::Bytes;
use http_body_util::{Either, Full};
use hyper::{Response, StatusCode};
use std::error::Error as StdError;
use ts_utils::bencode;

use super::response::Body;

type Cause = Box<dyn StdError + Send + Sync>;

/// The `HttpError` enum represents various HTTP-related error types that can occur during processing
/// of HTTP requests or responses.
#[allow(unused)]
#[derive(Debug)]
pub(super) enum HttpError {
    /// The requested resource was not found on the server (404 Not Found).
    NotFound,
    /// The server timed out while waiting for the request to be completed (408 Request Timeout).
    RequestTimeout,
    /// The request lacks valid authentication credentials (401 Unauthorized).
    Unauthorized,
    /// The request contains invalid data or parameters, along with an additional error message  (400 BadRequest).
    BadRequest(Cause),
    /// An HTTP error occurred, along with cause.
    Other(Cause),
}

/// Alias for a `Result` with the error type `Error`.
pub(super) type Result<T> = std::result::Result<T, HttpError>;

impl From<hyper::Error> for HttpError {
    fn from(err: hyper::Error) -> Self {
        HttpError::Other(err.into())
    }
}

impl std::fmt::Display for HttpError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            HttpError::NotFound => write!(f, "{}", constants::NOT_FOUND),
            HttpError::RequestTimeout => write!(f, "{}", constants::REQUEST_TIMEOUT),
            HttpError::Unauthorized => write!(f, "{}", constants::UNAUTHORIZED),
            HttpError::BadRequest(reason) => write!(f, "{}", reason),
            HttpError::Other(reason) => write!(f, "{}", reason),
        }
    }
}

impl StdError for HttpError {}

impl Into<Response<Body>> for HttpError {
    fn into(self) -> Response<Body> {
        let status_code = match self {
            HttpError::NotFound => StatusCode::NOT_FOUND,
            HttpError::BadRequest(_) => StatusCode::BAD_REQUEST,
            HttpError::RequestTimeout => StatusCode::REQUEST_TIMEOUT,
            HttpError::Unauthorized => StatusCode::UNAUTHORIZED,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };

        let body = Full::new(Bytes::from(self.to_string()));

        Response::builder()
            .status(status_code)
            .body(Either::Left(body))
            .unwrap()
    }
}

impl From<worker::Error> for HttpError {
    fn from(err: worker::Error) -> Self {
        HttpError::Other(err.into())
    }
}

impl From<bencode::Error> for HttpError {
    fn from(err: bencode::Error) -> Self {
        HttpError::Other(err.into())
    }
}
