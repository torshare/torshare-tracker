use hyper::Method;
use hyper::{service::Service, Body, Request, Response};
use log::debug;
use std::future::Future;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::{net::SocketAddr, pin::Pin};

use super::error::Error;
use super::response::HttpResponse;
use crate::worker::{Worker, WorkerTask};

pub(super) struct RequestHandler {
    pub addr: SocketAddr,
    pub worker: Arc<Worker>,
}

impl Service<Request<Body>> for RequestHandler {
    type Response = Response<Body>;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _: &mut Context) -> std::task::Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let worker = self.worker.clone();
        let addr = self.addr;

        Box::pin(async move {
            RequestHandler::handle_request(req, worker, addr)
                .await
                .map(Into::into)
                .or_else(|e| {
                    debug!("request failed: {:?}", e);
                    Ok(e.into())
                })
        })
    }
}

impl RequestHandler {
    async fn handle_request(req: Request<Body>, worker: Arc<Worker>, _addr: SocketAddr) -> Result<HttpResponse, Error> {
        println!("request received: {:?}", req);
        match (req.method(), req.uri().path()) {
            (&Method::GET, "/ping") => Ok(HttpResponse::from("pong")),
            (&Method::GET, "/announce") => {
                let _ = worker.work(WorkerTask::Announce).await?;
                Ok(HttpResponse::from("pong"))
            }
            _ => Err(Error::NotFound),
        }
    }
}
