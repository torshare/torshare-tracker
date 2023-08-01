mod error;
mod handler;
mod response;

use hyper::server::conn::AddrStream;
use hyper::service::make_service_fn;
use hyper::Server;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;

use self::handler::RequestHandler;
use crate::signals::global_shutdown_signal;
use crate::worker::Worker;

pub struct HttpServer {
    worker: Arc<Worker>,
}

impl HttpServer {
    pub fn new(worker: Arc<Worker>) -> HttpServer {
        HttpServer { worker }
    }

    pub async fn start(&self) {
        // Construct our SocketAddr to listen on...
        let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

        // And a MakeService to handle each connection...
        let make_service = make_service_fn(|socket: &AddrStream| {
            let addr = socket.remote_addr();
            let worker = self.worker.clone();
            async move { Ok::<_, Infallible>(RequestHandler { addr, worker }) }
        });

        // Then bind and serve...
        let server = Server::bind(&addr).serve(make_service);

        // And now add a graceful shutdown signal...
        let graceful = server.with_graceful_shutdown(global_shutdown_signal());

        // And run forever...
        if let Err(e) = graceful.await {
            eprintln!("server error: {}", e);
        }
    }
}
