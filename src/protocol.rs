use crate::hub::{ConnectError, Hub, Update};
use anyhow::{anyhow, Context, Result};
use futures_util::{SinkExt, StreamExt};
use std::net::{IpAddr, SocketAddr};
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream};
use tokio::time::timeout;
use tokio_util::codec::{Framed, LinesCodec};
use tracing::{debug, info, warn};

const AUTH_TIMEOUT: Duration = Duration::from_secs(10);
const IDLE_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_LINE: usize = 64 * 1024;

pub async fn run(listener: TcpListener, hub: Arc<RwLock<Hub>>) {
    let addr = listener
        .local_addr()
        .map(|addr| addr.to_string())
        .unwrap_or_else(|_| "<unknown>".to_string());
    info!(%addr, "tcp listening");

    loop {
        match listener.accept().await {
            Ok((stream, peer)) => {
                let hub = hub.clone();
                tokio::spawn(async move {
                    if let Err(err) = handle(stream, peer, hub).await {
                        debug!(%peer, error = %err, "client disconnected");
                    }
                });
            }
            Err(err) => {
                warn!(error = %err, "tcp accept failed");
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }
    }
}

async fn handle(stream: TcpStream, peer: SocketAddr, hub: Arc<RwLock<Hub>>) -> Result<()> {
    let _ = stream.set_nodelay(true);
    let via = family(&peer);
    let mut framed = Framed::new(stream, LinesCodec::new_with_max_length(MAX_LINE));

    framed
        .send("Authentication required:".to_string())
        .await
        .context("send auth greeting")?;

    let line = timeout(AUTH_TIMEOUT, framed.next())
        .await
        .context("auth timeout")?
        .context("connection closed")??;

    let Some((username, password)) = line.split_once(':') else {
        let _ = framed
            .send("Authentication failed. Access denied.".to_string())
            .await;
        warn!(%peer, "malformed authentication line");
        return Ok(());
    };

    let auth = hub.read().unwrap().authenticate(username.trim(), password);
    let Some((index, id, username)) = auth else {
        let _ = framed
            .send("Authentication failed. Access denied.".to_string())
            .await;
        warn!(%peer, "authentication failed");
        return Ok(());
    };

    // Do not hold the lock guard across the await below.
    let connect_result = hub.write().unwrap().connect(index, via);
    match connect_result {
        Ok(()) => {}
        Err(ConnectError::AlreadyConnected) => {
            let _ = framed
                .send("Only one connection per user allowed.".to_string())
                .await;
            return Ok(());
        }
        Err(ConnectError::Unknown) => return Err(anyhow!("unknown node")),
    }

    let result = async {
        framed
            .send("Authentication successful. Access granted.".to_string())
            .await?;
        let mode = if via == "v6" {
            "You are connecting via: IPv6"
        } else {
            "You are connecting via: IPv4"
        };
        framed.send(mode.to_string()).await?;
        info!(%peer, %username, via, %id, "client connected");

        loop {
            let line = timeout(IDLE_TIMEOUT, framed.next())
                .await
                .context("idle timeout")?
                .context("connection closed")??;
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            if let Some(payload) = line.strip_prefix("update ") {
                let payload = payload.trim();
                if payload.is_empty() {
                    continue;
                }
                match serde_json::from_str::<Update>(payload) {
                    Ok(update) => hub.write().unwrap().update(index, &update),
                    Err(err) => warn!(%username, error = %err, "invalid update json"),
                }
            } else if line == "logout" {
                info!(%username, "client logout");
                return Ok(());
            } else if line.starts_with("pong") {
                continue;
            } else {
                debug!(%username, command = %line, "unknown command");
            }
        }
    }
    .await;

    hub.write().unwrap().disconnect(index);
    result
}

fn family(peer: &SocketAddr) -> &'static str {
    match peer.ip() {
        IpAddr::V4(_) => "v4",
        IpAddr::V6(ip) if ip.to_ipv4_mapped().is_some() => "v4",
        IpAddr::V6(_) => "v6",
    }
}
