mod error;
mod handler;
mod request;
mod response;

use hyper::server::conn::http1;
use hyper_util::rt::TokioIo;
use log::{debug, error, info};
use socket2::{Protocol, Socket};
use std::{
    net::{IpAddr, SocketAddr},
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::{
    net::TcpListener,
    sync::{mpsc, Semaphore},
    time::sleep_until,
};

use self::handler::Handler;
use super::State;
use crate::signals::StopSignalRx;

pub struct HttpServer {
    state: State,
}

impl HttpServer {
    pub fn new(state: State) -> HttpServer {
        HttpServer { state }
    }

    pub async fn start(
        &self,
        mut stop_signal_rx: StopSignalRx,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let config = &self.state.config;
        let port = config.http_port();
        let ip: IpAddr = config.http_host().parse()?;

        let domain = if ip.is_ipv6() {
            socket2::Domain::IPV6
        } else {
            socket2::Domain::IPV4
        };

        let socket = Socket::new(domain, socket2::Type::STREAM, Some(Protocol::TCP))?;
        let addr = SocketAddr::from((ip, port));
        let sock_addr = addr.into();

        #[cfg(unix)]
        socket.set_reuse_port(true)?;

        socket.bind(&sock_addr)?;
        socket.listen(config.connection_backlog_size())?;

        let listener: std::net::TcpListener = socket.into();
        listener.set_nonblocking(true)?;
        let listener = TcpListener::from_std(listener)?;

        info!("Listening on http://{}", addr);

        let state = self.state.clone();
        let rx = stop_signal_rx.clone();

        let task = tokio::spawn(async move {
            if let Err(e) = accept_loop(listener, state, rx).await {
                error!("server error: {}", e);
            }
        });

        let _ = stop_signal_rx.changed().await?;

        info!("Shutting down http server...");

        let _ = task.abort();
        let _ = task.await;

        Ok(())
    }
}

// hyper/src/server/conn/http1.rs:298:9
const MINIMUM_MAX_BUFFER_SIZE: usize = 8192;

async fn accept_loop(
    listener: TcpListener,
    state: State,
    stop_global_signal_rx: StopSignalRx,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let max_buffer_size =
        std::cmp::max(MINIMUM_MAX_BUFFER_SIZE, state.config.max_read_buffer_size());

    let is_keep_alive_enabled = state.config.is_keep_alive_enabled();
    let max_open_connections = state.config.max_open_connections();
    let request_timeout = state.config.http_request_timeout();

    let semaphore = Arc::new(Semaphore::new(max_open_connections));

    loop {
        let permit = semaphore.clone().acquire_owned().await?;
        let (stream, addr) = listener.accept().await?;

        let io = TokioIo::new(stream);
        let mut stop_global_signal_rx = stop_global_signal_rx.clone();
        let state = state.clone();

        tokio::spawn(async move {
            let (reset_timer_tx, reset_timer_rx) = mpsc::channel(1);
            let reset_timer_tx = match is_keep_alive_enabled {
                true => Some(reset_timer_tx),
                false => None,
            };

            let handler = Handler::new(addr, state, reset_timer_tx);
            let connection = http1::Builder::new()
                .max_buf_size(max_buffer_size)
                .keep_alive(is_keep_alive_enabled)
                .serve_connection(io, handler);

            tokio::pin!(connection);

            tokio::select! {
                _ = stop_global_signal_rx.changed() => {
                    connection.graceful_shutdown();
                }

                _ = create_request_timer(request_timeout, reset_timer_rx) => {
                    drop(permit);
                    connection.graceful_shutdown();
                }

                res = &mut connection => {
                    drop(permit);

                    if let Err(err) = res {
                        match err.into_cause() {
                            Some(cause) => debug!("Error while serving connection: {}", cause),
                            None => {}
                        }
                    }
                }
            }
        });
    }
}

async fn create_request_timer(timeout_duration: Duration, mut reset_timer_rx: mpsc::Receiver<()>) {
    let deadline = Instant::now() + timeout_duration;
    let timeout_fut = sleep_until(deadline.into());
    tokio::pin!(timeout_fut);

    loop {
        tokio::select! {
            res = reset_timer_rx.recv() => match res {
                Some(_) => {
                    let deadline = Instant::now() + timeout_duration;
                    timeout_fut.as_mut().reset(deadline.into());
                },
                None => {},
            },

            _ = &mut timeout_fut => {
                break;
            }
        }
    }
}
