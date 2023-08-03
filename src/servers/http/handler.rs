use hyper::Method;
use hyper::{service::Service, Body, Request, Response};
use log::{debug, error, log_enabled, Level};
use std::future::Future;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::{net::SocketAddr, pin::Pin};

use super::error::HttpError;
use super::response::HttpResponse;
use crate::models::tracker::AnnounceRequest;
use crate::servers::http::request::HttpRequest;
use crate::worker::{Worker, WorkerResponse, WorkerTask};

pub(super) struct Handler {
    pub addr: SocketAddr,
    pub worker: Arc<Worker>,
}

impl Service<Request<Body>> for Handler {
    type Response = Response<Body>;
    type Error = HttpError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _: &mut Context) -> std::task::Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let worker = self.worker.clone();
        let addr = self.addr;

        Box::pin(async move {
            Handler::handle_request(req, worker, addr)
                .await
                .map(Into::into)
                .or_else(|e| {
                    if log_enabled!(Level::Error) {
                        error!("request failed: {:?}", e);
                    }

                    Ok(e.into())
                })
        })
    }
}

impl Handler {
    async fn handle_request(
        req: Request<Body>,
        worker: Arc<Worker>,
        addr: SocketAddr,
    ) -> Result<HttpResponse, HttpError> {
        if log_enabled!(Level::Debug) {
            debug!("request received: {:?}", req);
        }

        match (req.method(), req.uri().path()) {
            (&Method::GET, "/ping") => Ok(HttpResponse::from("pong")),
            (&Method::GET, "/announce") => announce(req, worker, addr).await,
            (&Method::GET, "/scrape") => Ok(HttpResponse::from("scrape")),
            _ => Err(HttpError::NotFound),
        }
    }
}

/// Handle an announce request.
async fn announce(req: Request<Body>, worker: Arc<Worker>, _addr: SocketAddr) -> Result<HttpResponse, HttpError> {
    let request: AnnounceRequest = req.query_params()?;
    let is_compact = request.compact;

    if log_enabled!(Level::Debug) {
        debug!("announce request received: {:?}", request);
    }

    match worker.work(WorkerTask::Announce(request)).await? {
        WorkerResponse::Announce(response) => {
            if log_enabled!(Level::Debug) {
                debug!("announce response: {:?}", response);
            }

            let output = match is_compact {
                true => response.compact(),
                _ => response.non_compact(),
            };

            Ok(HttpResponse::from(output))
        }
        _ => unreachable!(),
    }
}
