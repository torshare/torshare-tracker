use crate::{
    constants::{NOT_FOUND, REQUEST_TIMEOUT, UNAUTHORIZED},
    worker::WorkerError,
};
use hyper::{Body, Response, StatusCode};
use std::error::Error as StdError;

#[allow(unused)]
#[derive(Debug)]
pub(super) enum Error {
    NotFound,
    RequestTimeout,
    Unauthorized,
    BadRequest(String),
    Other(String),
}

#[allow(unused)]
/// Alias for a `Result` with the error type `Error`.
pub(super) type Result<T> = std::result::Result<T, Error>;

impl From<hyper::Error> for Error {
    fn from(err: hyper::Error) -> Self {
        Error::Other(err.to_string())
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::NotFound => write!(f, "{}", NOT_FOUND),
            Error::RequestTimeout => write!(f, "{}", REQUEST_TIMEOUT),
            Error::Unauthorized => write!(f, "{}", UNAUTHORIZED),
            Error::BadRequest(reason) => write!(f, "{}", reason),
            Error::Other(reason) => write!(f, "{}", reason),
        }
    }
}

impl StdError for Error {}

impl Into<Response<Body>> for Error {
    fn into(self) -> Response<Body> {
        let status_code = match self {
            Error::NotFound => StatusCode::NOT_FOUND,
            Error::BadRequest(_) => StatusCode::BAD_REQUEST,
            Error::RequestTimeout => StatusCode::REQUEST_TIMEOUT,
            Error::Unauthorized => StatusCode::UNAUTHORIZED,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };

        Response::builder()
            .status(status_code)
            .body(Body::from(self.to_string()))
            .unwrap()
    }
}

impl From<WorkerError> for Error {
    fn from(err: WorkerError) -> Self {
        Error::Other(err.to_string())
    }
}
