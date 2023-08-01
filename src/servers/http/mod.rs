use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server};
use std::convert::Infallible;
use std::net::SocketAddr;

use crate::signals::shutdown_signal;

pub struct HttpServer {}

async fn handle(_req: Request<Body>) -> Result<Response<Body>, Infallible> {
    Ok(Response::new(Body::from("Hello World")))
}

impl HttpServer {
    pub async fn start() {
        // Construct our SocketAddr to listen on...
        let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

        // And a MakeService to handle each connection...
        let make_service = make_service_fn(|_conn| async { Ok::<_, Infallible>(service_fn(handle)) });

        // Then bind and serve...
        let server = Server::bind(&addr).serve(make_service);

        // And now add a graceful shutdown signal...
        let graceful = server.with_graceful_shutdown(shutdown_signal());

        // And run forever...
        if let Err(e) = graceful.await {
            eprintln!("server error: {}", e);
        }
    }
}
