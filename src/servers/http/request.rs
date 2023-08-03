use super::error::{HttpError, Result};
use ts_utils::query;

/// The `HttpRequest` trait represents an HTTP request and provides methods for accessing
/// various aspects of the request, such as query parameters.
///
/// This trait is intended to be implemented by HTTP request structs or types that contain
/// the necessary information about an incoming HTTP request.
pub(super) trait HttpRequest {
    /// Parses and deserializes the query parameters of the HTTP request into a given type `T`.
    /// The type `T` must implement `serde::Deserialize`.
    ///
    /// # Returns
    /// - `Ok(T)`: If the query parameters were successfully deserialized into the specified type `T`.
    /// - `Err`: If there was an error during deserialization or if the query parameters are invalid.
    fn query_params<'de, T: serde::Deserialize<'de>>(&'de self) -> Result<T>;
}

impl HttpRequest for hyper::Request<hyper::Body> {
    #[inline]
    fn query_params<'de, T: serde::Deserialize<'de>>(&'de self) -> Result<T> {
        let query = self.uri().query().unwrap_or_default();
        query::from_bytes(query.as_bytes()).map_err(|err| HttpError::BadRequest(err.to_string()))
    }
}
