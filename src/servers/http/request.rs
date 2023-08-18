use std::net::IpAddr;

use super::error::{HttpError, Result};
use crate::utils::Loggable;
use ts_utils::{query, string::get_first_value};

pub(super) struct HttpRequest<T>(pub hyper::Request<T>);

impl<T> std::ops::Deref for HttpRequest<T> {
    type Target = hyper::Request<T>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<Body> HttpRequest<Body> {
    /// Parses and deserializes the query parameters of the HTTP request into a given type `T`.
    /// The type `T` must implement `serde::Deserialize`.
    ///
    /// # Returns
    /// - `Ok(T)`: If the query parameters were successfully deserialized into the specified type `T`.
    /// - `Err`: If there was an error during deserialization or if the query parameters are invalid.
    pub fn query_params<'de, T: serde::Deserialize<'de>>(&'de self) -> Result<T> {
        let query = self.uri().query().unwrap_or_default();
        query::from_bytes(query.as_bytes()).map_err(|err| HttpError::BadRequest(err.into()))
    }

    /// This function extracts an IP address from an HTTP header with the specified name.
    ///
    /// # Arguments
    ///
    /// * `header_name` - The name of the HTTP header containing the IP address.
    ///
    /// # Returns
    ///
    /// Returns an `Option<IpAddr>` representing the parsed IP address if successful,
    /// or `None` if the header value couldn't be parsed into an IP address.
    pub fn reverse_ip(&self, header_name: &str) -> Option<IpAddr> {
        self.headers()
            .get(header_name)
            .and_then(|header| header.to_str().ok())
            .and_then(|header| get_first_value(header, ',').parse().ok())
    }
}

impl<T> Loggable for HttpRequest<T> {
    fn log(&self) -> String {
        let uri = self.uri();
        let method = self.method();
        let version = self.version();
        let headers = self.headers();

        format!(
            "Request: {:?} {} {} | Headers: {:?}",
            version, method, uri, headers
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hyper::http::Uri;
    use hyper::Method;

    #[test]
    fn test_reverse_ip() {
        let uri = "/test".parse::<Uri>().unwrap();
        let request = HttpRequest(
            hyper::Request::builder()
                .method(Method::GET)
                .uri(uri)
                .version(hyper::Version::HTTP_11)
                .header("X-Forwarded-For", "192.168.1.1")
                .body(())
                .unwrap(),
        );

        assert_eq!(
            request.reverse_ip("X-Forwarded-For"),
            Some(IpAddr::from([192, 168, 1, 1]))
        );
    }

    #[test]
    fn test_query_params() {
        let uri = "/test?foo=bar&baz=qux".parse::<Uri>().unwrap();
        let request = HttpRequest(
            hyper::Request::builder()
                .method(Method::GET)
                .uri(uri)
                .version(hyper::Version::HTTP_11)
                .body(())
                .unwrap(),
        );

        #[derive(serde::Deserialize, Debug, PartialEq)]
        struct Query {
            foo: String,
            baz: String,
        }

        let query_params = request.query_params();
        assert!(query_params.is_ok());

        let query_params: Query = query_params.unwrap();

        assert_eq!(query_params.foo, "bar".to_string());
        assert_eq!(query_params.baz, "qux".to_string());
    }
}
