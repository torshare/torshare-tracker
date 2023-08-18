use super::error::HttpError;
use super::response::{Body, BodyStream, HttpResponse};
use crate::constants;
use crate::models::tracker::{
    AnnounceRequest, AnnounceResponse, ScrapeRequest, ScrapeResponse, TrackerError,
};
use crate::servers::cache::full_scrape;
use crate::servers::http::request::HttpRequest;
use crate::servers::State;
use crate::utils::Loggable;
use crate::worker::Task;

use bytes::Bytes;
use hyper::Method;
use hyper::{body::Incoming as IncomingBody, service::Service, Request, Response};
use log::{debug, info, log_enabled, Level};
use std::future::Future;
use std::{net::SocketAddr, pin::Pin};
use tokio::sync::mpsc;

pub(super) struct Handler {
    addr: SocketAddr,
    state: State,
    on_response_finish: Option<mpsc::Sender<()>>,
}

impl Service<Request<IncomingBody>> for Handler {
    type Response = Response<Body>;
    type Error = std::convert::Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn call(&self, req: Request<IncomingBody>) -> Self::Future {
        let state = self.state.clone();
        let addr = self.addr;
        let req = HttpRequest(req);
        let tx = self.on_response_finish.clone();

        Box::pin(async move {
            let res = Handler::handle_request(req, state, addr)
                .await
                .map(Into::into)
                .or_else(request_error_handler);

            if let Some(tx) = tx {
                let _ = tx.send(()).await;
            }

            Ok(res?)
        })
    }
}

impl Handler {
    pub fn new(
        addr: SocketAddr,
        state: State,
        on_response_finish: Option<mpsc::Sender<()>>,
    ) -> Handler {
        Handler {
            addr,
            state,
            on_response_finish,
        }
    }

    async fn handle_request(
        req: HttpRequest<IncomingBody>,
        state: State,
        addr: SocketAddr,
    ) -> Result<HttpResponse, HttpError> {
        if log_enabled!(Level::Info) && state.config.http_log_request() {
            info!("{}", req.log());
        }

        match (req.method(), req.uri().path()) {
            (&Method::GET, "/ping") => Ok(HttpResponse::from("pong")),
            (&Method::GET, "/announce") => announce(req, state, addr).await.or_else(|err| {
                debug!("announce failed: {:?}", err);
                convert_to_tracker_response(err)
            }),
            (&Method::GET, "/scrape") => scrape(req, state).await.or_else(|err| {
                debug!("scrape failed: {:?}", err);
                convert_to_tracker_response(err)
            }),
            _ => Err(HttpError::NotFound),
        }
    }
}

async fn announce(
    req: HttpRequest<IncomingBody>,
    state: State,
    addr: SocketAddr,
) -> Result<HttpResponse, HttpError> {
    if !state.config.allow_http_announce() {
        let err: TrackerError = constants::TRACKER_ERROR_HTTP_ANNOUNCE_NOT_ALLOWED.into();
        return HttpResponse::try_from(err);
    }

    let request: AnnounceRequest = req.query_params()?;
    if log_enabled!(Level::Debug) {
        debug!("{}", request.log());
    }

    let ip_addr = match state.config.ip_forward_header_name() {
        Some(header_name) => req.reverse_ip(header_name),
        _ => None,
    }
    .unwrap_or_else(|| addr.ip());

    let task = Task::Announce((request, ip_addr));
    let response: AnnounceResponse = state.worker.work(task).await?.into();

    if log_enabled!(Level::Debug) {
        debug!("{}", response.log());
    }

    HttpResponse::try_from(response)
}

async fn scrape(req: HttpRequest<IncomingBody>, state: State) -> Result<HttpResponse, HttpError> {
    if !state.config.allow_http_scrape() {
        let err: TrackerError = constants::TRACKER_ERROR_HTTP_SCRAPE_NOT_ALLOWED.into();
        return HttpResponse::try_from(err);
    }

    let request: ScrapeRequest = req.query_params()?;

    if request.info_hashes.is_empty() {
        return full_scrape(state).await;
    }

    let task = Task::Scrape(request);
    let response: ScrapeResponse = state.worker.work(task).await?.into();

    HttpResponse::try_from(response)
}

async fn full_scrape(state: State) -> Result<HttpResponse, HttpError> {
    if !state.config.allow_full_scrape() {
        let err: TrackerError = constants::TRACKER_ERROR_FULL_SCRAPE_NOT_ALLOWED.into();
        return HttpResponse::try_from(err);
    }

    let cache = state.cache.full_scrape.read().await;
    let is_cache_expired = cache.is_expired() || cache.is_none();

    if is_cache_expired && !cache.is_refreshing() {
        let state = state.clone();
        let expires_in = state.config.full_scrape_cache_ttl();

        tokio::spawn(async move {
            full_scrape::refresh(state.cache, state.worker, expires_in).await;
        });
    }

    match cache.as_ref() {
        Some(val) => {
            let stream = BodyStream::from(val.clone());
            return Ok(HttpResponse::from(stream));
        }
        _ => HttpResponse::try_from(ScrapeResponse::default()),
    }
}

impl TryFrom<TrackerError> for HttpResponse {
    type Error = HttpError;

    fn try_from(err: TrackerError) -> Result<Self, Self::Error> {
        let bytes: Bytes = err.try_into()?;
        Ok(HttpResponse::from(bytes))
    }
}

impl TryFrom<AnnounceResponse> for HttpResponse {
    type Error = HttpError;

    fn try_from(response: AnnounceResponse) -> Result<Self, Self::Error> {
        let bytes: Bytes = response.try_into()?;
        Ok(HttpResponse::from(bytes))
    }
}

impl TryFrom<ScrapeResponse> for HttpResponse {
    type Error = HttpError;

    fn try_from(response: ScrapeResponse) -> Result<Self, Self::Error> {
        let bytes: Bytes = response.try_into()?;
        Ok(HttpResponse::from(bytes))
    }
}

fn request_error_handler(err: HttpError) -> Result<Response<Body>, std::convert::Infallible> {
    debug!("request failed: {:?}", err);
    Ok(err.into())
}

fn convert_to_tracker_response(err: HttpError) -> Result<HttpResponse, HttpError> {
    let err: TrackerError = err.to_string().into();
    return HttpResponse::try_from(err);
}
