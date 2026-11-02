use crate::config::{Config, Site};
use crate::hub::{now_unix, Hub, Node};
use async_stream::stream;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{header, HeaderValue, StatusCode, Uri};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use futures_util::Stream;
use rust_embed::RustEmbed;
use serde::Serialize;
use serde_json::json;
use std::convert::Infallible;
use std::sync::{Arc, RwLock};
use std::time::Duration;

#[derive(Clone)]
pub struct AppState {
    pub cfg: Arc<Config>,
    pub hub: Arc<RwLock<Hub>>,
}

#[derive(Serialize)]
pub struct StatusResponse {
    pub site: Site,
    pub updated: i64,
    pub servers: Vec<Node>,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/status", get(status))
        .route("/api/server/{id}", get(server))
        .route("/api/events", get(events))
        .route("/api/health", get(health))
        .route("/healthz", get(health))
        .route("/json/stats.json", get(legacy))
        .fallback(static_handler)
        .with_state(state)
}

async fn status(State(state): State<AppState>) -> Json<StatusResponse> {
    Json(snapshot(&state))
}

async fn server(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Response {
    match state.hub.read().unwrap().find(&id) {
        Some(node) => Json(node).into_response(),
        None => (StatusCode::NOT_FOUND, "node not found").into_response(),
    }
}

async fn health() -> Json<serde_json::Value> {
    Json(json!({
        "status": "ok",
        "time": now_unix(),
    }))
}

async fn events(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let stream = stream! {
        loop {
            yield Ok::<_, Infallible>(Event::default().data(snapshot_json(&state)));
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    };
    Sse::new(stream).keep_alive(KeepAlive::default())
}

fn snapshot(state: &AppState) -> StatusResponse {
    StatusResponse {
        site: state.cfg.site.clone(),
        updated: now_unix(),
        servers: state.hub.read().unwrap().snapshot(),
    }
}

fn snapshot_json(state: &AppState) -> String {
    serde_json::to_string(&snapshot(state)).unwrap_or_else(|_| "{}".to_string())
}

async fn legacy(State(state): State<AppState>) -> Json<serde_json::Value> {
    let nodes = state.hub.read().unwrap().snapshot();
    let servers: Vec<serde_json::Value> = nodes
        .iter()
        .map(|node| {
            if node.connected {
                json!({
                    "name": node.name,
                    "type": node.node_type,
                    "host": node.host,
                    "location": node.location,
                    "online4": node.online4,
                    "online6": node.online6,
                    "uptime": format_uptime(node.uptime),
                    "load": node.load,
                    "network_rx": node.network_rx,
                    "network_tx": node.network_tx,
                    "network_in": node.network_in,
                    "network_out": node.network_out,
                    "cpu": node.cpu,
                    "memory_total": node.memory_total,
                    "memory_used": node.memory_used,
                    "swap_total": node.swap_total,
                    "swap_used": node.swap_used,
                    "hdd_total": node.hdd_total,
                    "hdd_used": node.hdd_used,
                    "custom": node.custom,
                })
            } else {
                json!({
                    "name": node.name,
                    "type": node.node_type,
                    "host": node.host,
                    "location": node.location,
                    "online4": node.online4,
                    "online6": node.online6,
                })
            }
        })
        .collect();

    Json(json!({
        "servers": servers,
        "updated": now_unix(),
    }))
}

fn format_uptime(seconds: i64) -> String {
    let days = seconds / 86400;
    if days > 0 {
        return format!("{days} 天");
    }
    format!(
        "{:02}:{:02}:{:02}",
        seconds / 3600,
        (seconds / 60) % 60,
        seconds % 60
    )
}

#[derive(RustEmbed)]
#[folder = "web/dist/"]
struct Assets;

async fn static_handler(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    if path == "api" || path.starts_with("api/") {
        return StatusCode::NOT_FOUND.into_response();
    }
    let path = if path.is_empty() { "index.html" } else { path };

    match Assets::get(path) {
        Some(file) => asset_response(path, file),
        None => match Assets::get("index.html") {
            Some(file) => asset_response("index.html", file),
            None => (StatusCode::NOT_FOUND, "frontend not built").into_response(),
        },
    }
}

fn asset_response(path: &str, file: rust_embed::EmbeddedFile) -> Response {
    let mime = mime_guess::from_path(path).first_or_octet_stream();
    let mut response = Response::new(Body::from(file.data.into_owned()));
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_str(mime.as_ref())
            .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream")),
    );
    let cache = if path.starts_with("assets/") {
        "public, max-age=31536000, immutable"
    } else {
        "no-cache"
    };
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static(cache));
    response
}
