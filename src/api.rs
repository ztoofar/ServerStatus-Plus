use crate::config::{Config, Site};
use crate::hub::{Hub, Node, now_unix};
use async_stream::stream;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{HeaderValue, Method, StatusCode, Uri, header};
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

async fn server(State(state): State<AppState>, Path(id): Path<String>) -> Response {
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

async fn legacy(State(state): State<AppState>) -> Response {
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
                    "cpu": node.cpu as i64,
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
                    "online4": false,
                    "online6": false,
                })
            }
        })
        .collect();

    let mut response = Json(json!({
        "servers": servers,
        "updated": now_unix().to_string(),
    }))
    .into_response();

    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("no-store, no-cache, must-revalidate"),
    );
    response
        .headers_mut()
        .insert(header::PRAGMA, HeaderValue::from_static("no-cache"));
    response
        .headers_mut()
        .insert(header::EXPIRES, HeaderValue::from_static("0"));

    response
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
#[folder = "web/"]
struct Assets;

const DEFAULT_ANNOUNCEMENT: &str = "欢迎使用 ServerStatus-Plus";

async fn static_handler(State(state): State<AppState>, method: Method, uri: Uri) -> Response {
    if method != Method::GET && method != Method::HEAD {
        let mut response = (StatusCode::METHOD_NOT_ALLOWED, "method not allowed").into_response();
        response
            .headers_mut()
            .insert(header::ALLOW, HeaderValue::from_static("GET, HEAD"));
        return response;
    }

    let Some(path) = sanitize_asset_path(uri.path()) else {
        return not_found();
    };
    if path == "api" || path.starts_with("api/") {
        return not_found();
    }

    let Some(file) = Assets::get(&path) else {
        return not_found();
    };

    let is_head = method == Method::HEAD;
    let is_index = path == "index.html";
    let data = if is_index {
        render_index(&state, &file.data)
    } else {
        file.data.into_owned()
    };

    asset_response(&path, data, is_head, is_index)
}

fn sanitize_asset_path(raw: &str) -> Option<String> {
    let path = raw.strip_prefix('/')?;
    if path.is_empty() {
        return Some("index.html".to_string());
    }
    if path.len() > 512 || path.contains('\\') || path.contains('\0') || path.contains('%') {
        return None;
    }

    let mut normalized = String::with_capacity(path.len());
    for (index, segment) in path.split('/').enumerate() {
        if segment.is_empty()
            || segment == "."
            || segment == ".."
            || segment.starts_with('.')
            || segment.chars().any(|c| c.is_control())
        {
            return None;
        }
        if index > 0 {
            normalized.push('/');
        }
        normalized.push_str(segment);
    }
    Some(normalized)
}

fn not_found() -> Response {
    (StatusCode::NOT_FOUND, "not found").into_response()
}

fn render_index(state: &AppState, data: &[u8]) -> Vec<u8> {
    let template = String::from_utf8_lossy(data);
    let title = html_escape(state.cfg.site.title.trim());
    let announcement = if state.cfg.site.announcement.trim().is_empty() {
        DEFAULT_ANNOUNCEMENT.to_string()
    } else {
        state.cfg.site.announcement.trim().to_string()
    };

    template
        .replace("{{SITE_TITLE}}", &title)
        .replace("{{SITE_ANNOUNCEMENT}}", &html_escape(&announcement))
        .into_bytes()
}

fn html_escape(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(ch),
        }
    }
    out
}

fn asset_response(path: &str, data: Vec<u8>, is_head: bool, is_index: bool) -> Response {
    let length = data.len();
    let body = if is_head {
        Body::empty()
    } else {
        Body::from(data)
    };
    let mut response = Response::new(body);

    response
        .headers_mut()
        .insert(header::CONTENT_TYPE, content_type(path));
    response.headers_mut().insert(
        header::CONTENT_LENGTH,
        HeaderValue::from_str(&length.to_string())
            .unwrap_or_else(|_| HeaderValue::from_static("0")),
    );

    let cache = if is_index {
        "no-store, no-cache, must-revalidate"
    } else {
        "no-cache"
    };
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static(cache));

    response
}

fn content_type(path: &str) -> HeaderValue {
    let extension = path.rsplit('.').next().unwrap_or("");
    let value = match extension {
        "html" => "text/html; charset=utf-8".to_string(),
        "css" => "text/css; charset=utf-8".to_string(),
        "js" => "application/javascript; charset=utf-8".to_string(),
        "json" => "application/json; charset=utf-8".to_string(),
        "txt" => "text/plain; charset=utf-8".to_string(),
        "png" => "image/png".to_string(),
        "ico" => "image/x-icon".to_string(),
        _ => mime_guess::from_path(path)
            .first_or_octet_stream()
            .to_string(),
    };

    HeaderValue::from_str(&value)
        .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream"))
}
