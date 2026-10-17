//! Companion Server: HTTP API v1 + Socket.IO realtime on port 9863.
//!
//! Wire-compatible with the upstream YTMDesktop companion API (routes, payloads, error
//! codes, auth flow, rate limits) so existing companion apps keep working.

use crate::state_store::AuthToken;
use crate::ytm::{self, bridge::Playlist, Command};
use crate::{app_ctx, windows};
use axum::extract::{ConnectInfo, Json, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::Router;
use rand::{Rng, RngCore};
use regex::Regex;
use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use socketioxide::extract::{Data, SocketRef};
use socketioxide::handler::ConnectHandler;
use socketioxide::{SocketIo, TransportType};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Listener, Manager};
use tokio::sync::oneshot;

const LISTEN: &str = "0.0.0.0:9863";
const REALTIME_NS: &str = "/api/v1/realtime";
const MAX_AUTH_WINDOWS: usize = 5;
const AUTH_ENABLE_WINDOW: Duration = Duration::from_secs(300);

struct Running {
    io: SocketIo,
    shutdown: oneshot::Sender<()>,
}

struct PendingCode {
    app_id: String,
    app_name: String,
    app_version: String,
    expires: Instant,
}

pub struct AuthRequestInfo {
    pub app_name: String,
    pub code: String,
    result: Option<oneshot::Sender<bool>>,
}

/// Companion server runtime state, owned by the app context.
#[derive(Default)]
pub struct Companion {
    running: Mutex<Option<Running>>,
    codes: Mutex<HashMap<String, PendingCode>>,
    auth_requests: Mutex<HashMap<String, AuthRequestInfo>>,
    playlist_requests: Mutex<HashMap<String, oneshot::Sender<Vec<Playlist>>>>,
    limits: Mutex<HashMap<(&'static str, String), (Instant, u32)>>,
    auth_enable_generation: Mutex<u64>,
}

#[derive(Debug)]
pub struct ApiError {
    status: StatusCode,
    code: &'static str,
    message: String,
}

impl ApiError {
    fn new(status: u16, code: &'static str, message: impl Into<String>) -> Self {
        Self { status: StatusCode::from_u16(status).unwrap(), code, message: message.into() }
    }
    fn validation(message: impl Into<String>) -> Self {
        Self::new(400, "FST_ERR_VALIDATION", message)
    }
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = json!({
            "statusCode": self.status.as_u16(),
            "code": self.code,
            "error": self.status.canonical_reason().unwrap_or_default(),
            "message": self.message,
        });
        (self.status, Json(body)).into_response()
    }
}

fn unauthorized() -> Response {
    (StatusCode::UNAUTHORIZED, Json(json!({ "error": "UNAUTHORIZED" }))).into_response()
}

fn too_many(window: Duration) -> Response {
    let secs = window.as_secs();
    let when = if secs >= 60 { format!("{} minute", secs / 60) } else { format!("{secs} seconds") };
    (
        StatusCode::TOO_MANY_REQUESTS,
        Json(json!({ "statusCode": 429, "error": "Too Many Requests", "message": format!("Rate limit exceeded, retry in {when}") })),
    )
        .into_response()
}

fn hash_token(token: &str) -> String {
    hex::encode(Sha256::digest(token.as_bytes()))
}

impl Companion {
    /// Fixed-window limiter keyed by route and caller (token id or IP).
    fn allow(&self, route: &'static str, key: &str, max: u32, window: Duration) -> bool {
        let now = Instant::now();
        let mut limits = self.limits.lock().unwrap();
        limits.retain(|_, (start, _)| now.duration_since(*start) < Duration::from_secs(60));
        let entry = limits.entry((route, key.to_string())).or_insert((now, 0));
        if now.duration_since(entry.0) >= window {
            *entry = (now, 0);
        }
        entry.1 += 1;
        entry.1 <= max
    }

    pub fn auth_request_info(&self, request_id: &str) -> Option<(String, String)> {
        self.auth_requests.lock().unwrap().get(request_id).map(|r| (r.app_name.clone(), r.code.clone()))
    }

    pub fn resolve_auth_request(&self, request_id: &str, allowed: bool) {
        if let Some(tx) = self.auth_requests.lock().unwrap().get_mut(request_id).and_then(|r| r.result.take()) {
            let _ = tx.send(allowed);
        }
    }
}

fn token_valid(app: &AppHandle, token: &str) -> Option<String> {
    if token.is_empty() {
        return None;
    }
    let hash = hash_token(token);
    app_ctx(app).state.read(|s| s.companion_tokens.iter().find(|t| t.token_hash == hash).map(|t| t.id.clone()))
}

fn create_token(app: &AppHandle, app_id: &str, app_name: &str, app_version: &str) -> String {
    let mut bytes = [0u8; 256];
    rand::thread_rng().fill_bytes(&mut bytes);
    let token = hex::encode(bytes);
    let entry = AuthToken {
        app_id: app_id.into(),
        app_name: app_name.into(),
        app_version: app_version.into(),
        id: uuid::Uuid::new_v4().to_string(),
        token_hash: hash_token(&token),
    };
    app_ctx(app).state.update(|s| {
        s.companion_tokens.retain(|t| t.app_id != app_id);
        s.companion_tokens.push(entry);
    });
    token
}

/// Removes a token and drops its realtime connections.
pub fn revoke_token(app: &AppHandle, token_id: &str) {
    app_ctx(app).state.update(|s| s.companion_tokens.retain(|t| t.id != token_id));
    disconnect_invalid_sockets(app);
}

#[derive(Clone)]
struct TokenId(String);

fn disconnect_invalid_sockets(app: &AppHandle) {
    let ctx = app_ctx(app);
    let Some(io) = ctx.companion.running.lock().unwrap().as_ref().map(|r| r.io.clone()) else { return };
    let valid: Vec<String> = ctx.state.read(|s| s.companion_tokens.iter().map(|t| t.id.clone()).collect());
    if let Some(ns) = io.of(REALTIME_NS) {
        for socket in ns.sockets() {
            let id = socket.extensions.get::<TokenId>().map(|t| t.0);
            if !id.is_some_and(|id| valid.contains(&id)) {
                let _ = socket.disconnect();
            }
        }
    }
}

pub fn emit_realtime<T: Serialize + ?Sized>(app: &AppHandle, event: &'static str, data: &T) {
    let Some(io) = app_ctx(app).companion.running.lock().unwrap().as_ref().map(|r| r.io.clone()) else { return };
    let Ok(value) = serde_json::to_value(data) else { return };
    tauri::async_runtime::spawn(async move {
        if let Some(ns) = io.of(REALTIME_NS) {
            let _ = ns.emit(event, &value).await;
        }
    });
}

pub fn resolve_playlists(app: &AppHandle, request_id: &str, playlists: Vec<Playlist>) {
    if let Some(tx) = app_ctx(app).companion.playlist_requests.lock().unwrap().remove(request_id) {
        let _ = tx.send(playlists);
    }
}

/// Enables companion authorization requests; they switch off after 5 minutes or one success.
pub fn set_auth_window_enabled(app: &AppHandle, enabled: bool) {
    let ctx = app_ctx(app);
    let generation = {
        let mut g = ctx.companion.auth_enable_generation.lock().unwrap();
        *g += 1;
        *g
    };
    ctx.memory.update(app, |m| m.companion_server_auth_window_enabled = enabled);
    if enabled {
        let app = app.clone();
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(AUTH_ENABLE_WINDOW).await;
            let ctx = app_ctx(&app);
            if *ctx.companion.auth_enable_generation.lock().unwrap() == generation {
                ctx.memory.update(&app, |m| m.companion_server_auth_window_enabled = false);
            }
        });
    }
}

fn auth_window_enabled(app: &AppHandle) -> bool {
    app_ctx(app).memory.read(|m| m.companion_server_auth_window_enabled)
}

static APP_ID: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[a-z0-9_\-]{2,32}$").unwrap());
static APP_NAME: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[\w\W]{2,48}$").unwrap());
static SEMVER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-((?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*)(?:\.(?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*))*))?(?:\+([0-9a-zA-Z-]+(?:\.[0-9a-zA-Z-]+)*))?$").unwrap()
});
static CODE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[0-9]{4}$").unwrap());

fn field<'a>(body: &'a Value, name: &str, pattern: &Regex) -> Result<&'a str, ApiError> {
    body.get(name)
        .and_then(Value::as_str)
        .filter(|v| pattern.is_match(v))
        .ok_or_else(|| ApiError::validation(format!("body/{name} is missing or invalid")))
}

fn number(data: Option<&Value>, min: f64, max: f64) -> Result<f64, ApiError> {
    data.and_then(Value::as_f64)
        .filter(|n| n.is_finite() && *n >= min && *n <= max)
        .ok_or_else(|| ApiError::validation("body/data is missing or out of range"))
}

/// Validates a `/command` body against the current player state (port of `sendCommand`).
pub fn parse_command(body: &Value, state: &crate::player::PlayerState) -> Result<Command, ApiError> {
    let name = body.get("command").and_then(Value::as_str).ok_or_else(|| ApiError::validation("body/command is required"))?;
    let data = body.get("data");
    if let Some(cmd) = Command::simple(name) {
        return Ok(cmd);
    }
    Ok(match name {
        "setVolume" => Command::SetVolume(number(data, 0.0, 100.0)?.round() as u8),
        "seekTo" => {
            let position = number(data, 0.0, f64::MAX)?;
            let duration = state.video.as_ref().map_or(0, |v| v.duration_seconds) as f64;
            if position > duration {
                return Err(ApiError::new(400, "INVALID_SEEK_POSITION", format!("Seek position '{position}' is invalid")));
            }
            Command::SeekTo(position)
        }
        "repeatMode" => match data.and_then(Value::as_i64) {
            Some(0) => Command::RepeatMode("NONE"),
            Some(1) => Command::RepeatMode("ALL"),
            Some(2) => Command::RepeatMode("ONE"),
            Some(-1) => return Err(ApiError::new(400, "INVALID_REPEAT_MODE", "Repeat mode '-1' cannot be set")),
            _ => return Err(ApiError::validation("body/data must be a repeat mode")),
        },
        "playQueueIndex" => {
            let index = number(data, 0.0, f64::MAX)? as usize;
            let len = state.player.queue.as_ref().map_or(0, |q| q.items.len() + q.automix_items.len());
            if index >= len {
                return Err(ApiError::new(400, "INVALID_QUEUE_INDEX", format!("'{index}' is an invalid queue index for the current queue")));
            }
            Command::PlayQueueIndex(index)
        }
        "changeVideo" => {
            let data = data.filter(|d| d.is_object()).ok_or_else(|| ApiError::validation("body/data must be an object"))?;
            let get = |k: &str| data.get(k).and_then(Value::as_str).map(str::to_string);
            let (video_id, playlist_id) = (get("videoId"), get("playlistId"));
            if video_id.is_none() && playlist_id.is_none() {
                return Err(ApiError::new(400, "INVALID_CHANGE_REQUEST", "'videoId', 'playlistId', or both must be provided"));
            }
            Command::ChangeVideo { video_id, playlist_id }
        }
        _ => return Err(ApiError::validation("body/command is not a known command")),
    })
}

type Ip = ConnectInfo<SocketAddr>;

fn authenticate(app: &AppHandle, headers: &HeaderMap) -> Result<String, Response> {
    let token = headers.get("authorization").and_then(|v| v.to_str().ok()).unwrap_or_default();
    token_valid(app, token).ok_or_else(unauthorized)
}

async fn request_code(State(app): State<AppHandle>, ConnectInfo(ip): Ip, Json(body): Json<Value>) -> Response {
    let ctx = app_ctx(&app);
    if !ctx.companion.allow("requestcode", &ip.ip().to_string(), 5, Duration::from_secs(60)) {
        return too_many(Duration::from_secs(60));
    }
    let result = (|| {
        let app_id = field(&body, "appId", &APP_ID)?;
        let app_name = field(&body, "appName", &APP_NAME)?;
        let app_version = field(&body, "appVersion", &SEMVER)?;
        if !auth_window_enabled(&app) {
            return Err(ApiError::new(403, "AUTHORIZATION_DISABLED", "Authorization requests are disabled"));
        }
        let mut codes = ctx.companion.codes.lock().unwrap();
        let now = Instant::now();
        codes.retain(|_, c| c.expires > now);
        let code = (0..100)
            .map(|_| format!("{:04}", rand::thread_rng().gen_range(0..10_000)))
            .find(|c| !codes.contains_key(c))
            .ok_or_else(|| ApiError::new(504, "AUTHORIZATION_TIME_OUT", "Authorization timed out"))?;
        codes.insert(
            code.clone(),
            PendingCode { app_id: app_id.into(), app_name: app_name.into(), app_version: app_version.into(), expires: now + Duration::from_secs(60) },
        );
        Ok(code)
    })();
    match result {
        Ok(code) => Json(json!({ "code": code })).into_response(),
        Err(e) => e.into_response(),
    }
}

async fn request_token(State(app): State<AppHandle>, ConnectInfo(ip): Ip, Json(body): Json<Value>) -> Response {
    let ctx = app_ctx(&app);
    if !ctx.companion.allow("request", &ip.ip().to_string(), 5, Duration::from_secs(60)) {
        return too_many(Duration::from_secs(60));
    }
    let (app_id, code) = match (field(&body, "appId", &APP_ID), field(&body, "code", &CODE)) {
        (Ok(a), Ok(c)) => (a.to_string(), c.to_string()),
        (Err(e), _) | (_, Err(e)) => return e.into_response(),
    };
    if ctx.companion.auth_requests.lock().unwrap().len() >= MAX_AUTH_WINDOWS {
        return ApiError::new(503, "AUTHORIZATION_TOO_MANY", "Too many authorization requests currently active").into_response();
    }
    if !auth_window_enabled(&app) {
        return ApiError::new(403, "AUTHORIZATION_DISABLED", "Authorization requests are disabled").into_response();
    }
    let pending = {
        let mut codes = ctx.companion.codes.lock().unwrap();
        match codes.get(&code) {
            Some(c) if c.app_id == app_id && c.expires > Instant::now() => codes.remove(&code),
            _ => None,
        }
    };
    let Some(pending) = pending else {
        return ApiError::new(400, "AUTHORIZATION_INVALID", "Authorization invalid").into_response();
    };

    let request_id = uuid::Uuid::new_v4().to_string();
    let (tx, rx) = oneshot::channel();
    ctx.companion
        .auth_requests
        .lock()
        .unwrap()
        .insert(request_id.clone(), AuthRequestInfo { app_name: pending.app_name.clone(), code, result: Some(tx) });

    let label = format!("{}{request_id}", windows::AUTH_PREFIX);
    let opened = windows::open_companion_auth(&app, &request_id);
    if let (Ok(()), Some(window)) = (&opened, app.get_webview_window(&label)) {
        let app2 = app.clone();
        let id = request_id.clone();
        window.once("tauri://destroyed", move |_| app_ctx(&app2).companion.resolve_auth_request(&id, false));
    }
    let authorized = opened.is_ok() && matches!(tokio::time::timeout(Duration::from_secs(30), rx).await, Ok(Ok(true)));

    ctx.companion.auth_requests.lock().unwrap().remove(&request_id);
    windows::close_window(&app, &label);

    if authorized {
        let token = create_token(&app, &pending.app_id, &pending.app_name, &pending.app_version);
        set_auth_window_enabled(&app, false);
        Json(json!({ "token": token })).into_response()
    } else {
        ApiError::new(403, "AUTHORIZATION_DENIED", "Authorization request denied").into_response()
    }
}

async fn state(State(app): State<AppHandle>, headers: HeaderMap) -> Response {
    let token_id = match authenticate(&app, &headers) {
        Ok(id) => id,
        Err(r) => return r,
    };
    // Clients should use the realtime socket; this is for initial fetches.
    if !app_ctx(&app).companion.allow("state", &token_id, 1, Duration::from_secs(5)) {
        return too_many(Duration::from_secs(5));
    }
    Json(&*app_ctx(&app).player.get()).into_response()
}

async fn command(State(app): State<AppHandle>, headers: HeaderMap, Json(body): Json<Value>) -> Response {
    let token_id = match authenticate(&app, &headers) {
        Ok(id) => id,
        Err(r) => return r,
    };
    if !app_ctx(&app).companion.allow("command", &token_id, 2, Duration::from_secs(1)) {
        return too_many(Duration::from_secs(1));
    }
    match parse_command(&body, &app_ctx(&app).player.get()) {
        Ok(cmd) => {
            ytm::execute(&app, &cmd);
            StatusCode::NO_CONTENT.into_response()
        }
        Err(e) => e.into_response(),
    }
}

async fn playlists(State(app): State<AppHandle>, headers: HeaderMap) -> Response {
    let token_id = match authenticate(&app, &headers) {
        Ok(id) => id,
        Err(r) => return r,
    };
    let ctx = app_ctx(&app);
    // Each call makes a real YTM request; clients should cache and use realtime events.
    if !ctx.companion.allow("playlists", &token_id, 1, Duration::from_secs(30)) {
        return too_many(Duration::from_secs(30));
    }
    if ytm::webview(&app).is_none() {
        return ApiError::new(503, "YOUTUBE_MUSIC_UNVAILABLE", "YouTube Music is currently unvailable").into_response();
    }
    let request_id = uuid::Uuid::new_v4().to_string();
    let (tx, rx) = oneshot::channel();
    ctx.companion.playlist_requests.lock().unwrap().insert(request_id.clone(), tx);
    ytm::call(&app, "getPlaylists", &[json!(request_id)]);
    match tokio::time::timeout(Duration::from_secs(30), rx).await {
        Ok(Ok(list)) => Json(list).into_response(),
        _ => {
            ctx.companion.playlist_requests.lock().unwrap().remove(&request_id);
            ApiError::new(504, "YOUTUBE_MUSIC_TIME_OUT", "Response from YouTube Music took too long").into_response()
        }
    }
}

async fn global_limit(State(app): State<AppHandle>, ConnectInfo(ip): Ip, req: axum::extract::Request, next: axum::middleware::Next) -> Response {
    if !app_ctx(&app).companion.allow("global", &ip.ip().to_string(), 100, Duration::from_secs(60)) {
        return too_many(Duration::from_secs(60));
    }
    next.run(req).await
}

fn router(app: AppHandle, cors_wildcard: bool, io_layer: socketioxide::layer::SocketIoLayer) -> Router {
    let mut router = Router::new()
        .route("/metadata", get(|| async { Json(json!({ "apiVersions": ["v1"] })) }))
        .route("/api/v1/auth/requestcode", post(request_code))
        .route("/api/v1/auth/request", post(request_token))
        .route("/api/v1/state", get(state))
        .route("/api/v1/command", post(command))
        .route("/api/v1/playlists", get(playlists))
        .layer(axum::middleware::from_fn_with_state(app.clone(), global_limit))
        .layer(io_layer)
        .with_state(app);
    if cors_wildcard {
        router = router.layer(tower_http::cors::CorsLayer::permissive());
    }
    router
}

pub async fn enable(app: &AppHandle) {
    let ctx = app_ctx(app);
    if ctx.companion.running.lock().unwrap().is_some() {
        return;
    }
    let cors = ctx.settings.get().integrations.companion_server_cors_wildcard_enabled;
    let (io_layer, io) = SocketIo::builder().transports([TransportType::Websocket]).build_layer();

    // Connections to the default namespace are refused, matching upstream.
    io.ns("/", async |s: SocketRef| {
        let _ = s.disconnect();
    });
    let auth_app = app.clone();
    let auth = move |s: SocketRef, Data(auth): Data<Value>| {
        let app = auth_app.clone();
        async move {
            let token = auth.get("token").and_then(Value::as_str).unwrap_or_default();
            match token_valid(&app, token) {
                Some(id) => {
                    s.extensions.insert(TokenId(id));
                    Ok(())
                }
                None => Err("Authentication not provided or invalid"),
            }
        }
    };
    io.ns(REALTIME_NS, (async |_s: SocketRef| {}).with(auth));

    let listener = match tokio::net::TcpListener::bind(LISTEN).await {
        Ok(l) => l,
        Err(e) => {
            log::error!("Companion server failed to bind {LISTEN}: {e}");
            ctx.memory.update(app, |m| m.companion_server_error = Some(format!("Could not listen on port 9863: {e}")));
            return;
        }
    };
    ctx.memory.update(app, |m| m.companion_server_error = None);

    let (shutdown, shutdown_rx) = oneshot::channel::<()>();
    let service = router(app.clone(), cors, io_layer).into_make_service_with_connect_info::<SocketAddr>();
    tauri::async_runtime::spawn(async move {
        let result = axum::serve(listener, service).with_graceful_shutdown(async move {
            let _ = shutdown_rx.await;
        });
        if let Err(e) = result.await {
            log::error!("Companion server stopped: {e}");
        }
    });

    // Push every player state change to realtime clients.
    let mut rx = ctx.player.subscribe();
    let state_io = io.clone();
    tauri::async_runtime::spawn(async move {
        while rx.changed().await.is_ok() {
            let state = rx.borrow_and_update().clone();
            let Some(ns) = state_io.of(REALTIME_NS) else { break };
            let _ = ns.emit("state-update", &*state).await;
        }
    });

    *ctx.companion.running.lock().unwrap() = Some(Running { io, shutdown });
    log::info!("Companion server listening on {LISTEN}");
}

pub async fn disable(app: &AppHandle) {
    let running = app_ctx(app).companion.running.lock().unwrap().take();
    if let Some(r) = running {
        if let Some(ns) = r.io.of(REALTIME_NS) {
            let _ = ns.disconnect().await;
        }
        r.io.delete_ns(REALTIME_NS);
        r.io.delete_ns("/");
        let _ = r.shutdown.send(());
        log::info!("Companion server stopped");
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizedCompanion {
    pub id: String,
    pub app_id: String,
    pub app_name: String,
    pub app_version: String,
}

pub fn authorized_companions(app: &AppHandle) -> Vec<AuthorizedCompanion> {
    app_ctx(app).state.read(|s| {
        s.companion_tokens
            .iter()
            .map(|t| AuthorizedCompanion { id: t.id.clone(), app_id: t.app_id.clone(), app_name: t.app_name.clone(), app_version: t.app_version.clone() })
            .collect()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::player::PlayerState;

    #[test]
    fn command_validation_matches_upstream_errors() {
        let state = PlayerState::default();
        assert_eq!(parse_command(&json!({"command":"next"}), &state).unwrap(), Command::Next);
        assert_eq!(parse_command(&json!({"command":"setVolume","data":42}), &state).unwrap(), Command::SetVolume(42));
        assert_eq!(parse_command(&json!({"command":"setVolume","data":101}), &state).unwrap_err().code, "FST_ERR_VALIDATION");
        assert_eq!(parse_command(&json!({"command":"seekTo","data":5}), &state).unwrap_err().code, "INVALID_SEEK_POSITION");
        assert_eq!(parse_command(&json!({"command":"repeatMode","data":-1}), &state).unwrap_err().code, "INVALID_REPEAT_MODE");
        assert_eq!(parse_command(&json!({"command":"repeatMode","data":2}), &state).unwrap(), Command::RepeatMode("ONE"));
        assert_eq!(parse_command(&json!({"command":"playQueueIndex","data":0}), &state).unwrap_err().code, "INVALID_QUEUE_INDEX");
        assert_eq!(parse_command(&json!({"command":"changeVideo","data":{}}), &state).unwrap_err().code, "INVALID_CHANGE_REQUEST");
        assert!(matches!(
            parse_command(&json!({"command":"changeVideo","data":{"videoId":"abc"}}), &state).unwrap(),
            Command::ChangeVideo { video_id: Some(_), playlist_id: None }
        ));
        assert_eq!(parse_command(&json!({"command":"rm -rf"}), &state).unwrap_err().code, "FST_ERR_VALIDATION");
    }

    #[test]
    fn request_fields_follow_upstream_patterns() {
        assert!(APP_ID.is_match("stream-deck_1"));
        assert!(!APP_ID.is_match("Bad Id"));
        assert!(SEMVER.is_match("1.2.3-beta.1"));
        assert!(!SEMVER.is_match("1.2"));
        assert!(CODE.is_match("0042") && !CODE.is_match("42"));
    }

    #[test]
    fn rate_limiter_blocks_after_max() {
        let c = Companion::default();
        assert!(c.allow("t", "k", 2, Duration::from_secs(1)));
        assert!(c.allow("t", "k", 2, Duration::from_secs(1)));
        assert!(!c.allow("t", "k", 2, Duration::from_secs(1)));
        assert!(c.allow("t", "other", 2, Duration::from_secs(1)));
    }
}
