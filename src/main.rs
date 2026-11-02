mod api;
mod config;
mod hub;
mod protocol;

use anyhow::{Context, Result};
use clap::Parser;
use socket2::{Domain, Protocol as SocketProtocol, Socket, Type};
use std::net::{SocketAddr, TcpListener as StdTcpListener};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use tokio::net::TcpListener;
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;

#[derive(Debug, Parser)]
#[command(name = "sergate", version, about = "ServerStatus-Plus backend")]
struct Args {
    #[arg(short, long, default_value = "config.json")]
    config: PathBuf,
    #[arg(short, long)]
    verbose: bool,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let args = Args::parse();
    let level = if args.verbose { "debug" } else { "info" };
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(level)),
        )
        .init();

    let cfg = config::Config::load(&args.config)?;
    let listen = cfg.listen.clone();
    let http_listen = cfg.http_listen.clone();
    let hub = Arc::new(RwLock::new(hub::Hub::new(&cfg.servers)));
    let state = api::AppState {
        cfg: Arc::new(cfg),
        hub: hub.clone(),
    };

    let tcp_listener = bind_dual_stack(&listen).with_context(|| format!("bind tcp {listen}"))?;
    let http_listener =
        bind_dual_stack(&http_listen).with_context(|| format!("bind http {http_listen}"))?;

    info!(%listen, %http_listen, "ServerStatus-Plus starting");

    let tcp_task = tokio::spawn(protocol::run(tcp_listener, hub));
    let app = api::router(state);
    let server = axum::serve(http_listener, app).with_graceful_shutdown(shutdown_signal());

    tokio::select! {
        result = server => {
            if let Err(err) = result {
                error!(error = %err, "http server failed");
            }
        }
        result = tcp_task => {
            match result {
                Ok(()) => info!("tcp server stopped"),
                Err(err) => error!(error = %err, "tcp server failed"),
            }
        }
    }

    Ok(())
}

fn bind_dual_stack(addr: &str) -> Result<TcpListener> {
    let wildcard = addr.starts_with(':');
    let v6_addr = if wildcard {
        format!("[::]{addr}")
    } else {
        addr.to_string()
    };

    match bind(&v6_addr) {
        Ok(listener) => Ok(listener),
        Err(err) if wildcard => {
            warn!(%addr, error = %err, "IPv6 dual-stack bind failed, falling back to IPv4");
            let v4_addr = format!("0.0.0.0{addr}");
            bind(&v4_addr).with_context(|| format!("bind tcp {v4_addr}"))
        }
        Err(err) => Err(err).with_context(|| format!("bind tcp {v6_addr}")),
    }
}

fn bind(addr: &str) -> Result<TcpListener> {
    let sockaddr: SocketAddr = addr
        .parse()
        .with_context(|| format!("invalid listen address {addr}"))?;
    let domain = if sockaddr.is_ipv6() {
        Domain::IPV6
    } else {
        Domain::IPV4
    };

    let socket = Socket::new(domain, Type::STREAM, Some(SocketProtocol::TCP))?;
    socket.set_reuse_address(true)?;
    if sockaddr.is_ipv6() {
        socket.set_only_v6(false)?;
    }
    socket.set_nonblocking(true)?;
    socket.bind(&sockaddr.into())?;
    socket.listen(1024)?;

    let std_listener: StdTcpListener = socket.into();
    Ok(TcpListener::from_std(std_listener)?)
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
