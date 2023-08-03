use crate::{
    constants::{NOT_FOUND, REQUEST_TIMEOUT, UNAUTHORIZED},
    worker::WorkerError,
};
use hyper::{Body, Response, StatusCode};
use std::error::Error as StdError;

#[allow(unused)]
#[derive(Debug)]
pub(super) enum HttpError {
    NotFound,
    RequestTimeout,
    Unauthorized,
    BadRequest(String),
    Other(String),
}

#[allow(unused)]
/// Alias for a `Result` with the error type `Error`.
pub(super) type Result<T> = std::result::Result<T, HttpError>;

impl From<hyper::Error> for HttpError {
    fn from(err: hyper::Error) -> Self {
        HttpError::Other(err.to_string())
    }
}

impl std::fmt::Display for HttpError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            HttpError::NotFound => write!(f, "{}", NOT_FOUND),
            HttpError::RequestTimeout => write!(f, "{}", REQUEST_TIMEOUT),
            HttpError::Unauthorized => write!(f, "{}", UNAUTHORIZED),
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

        Response::builder()
            .status(status_code)
            .body(Body::from(self.to_string()))
            .unwrap()
    }
}

impl From<WorkerError> for HttpError {
    fn from(err: WorkerError) -> Self {
        HttpError::Other(err.to_string())
    }
}
