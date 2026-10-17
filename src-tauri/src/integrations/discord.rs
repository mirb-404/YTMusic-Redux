//! Discord Rich Presence over Discord's local IPC socket (port of upstream minimal client).

use crate::app_ctx;
use crate::player::PlayerState;
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::AppHandle;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::time::{sleep_until, Instant};

const MAX_RETRIES: u32 = 30;
const RETRY_DELAY: Duration = Duration::from_secs(5);
const DEBOUNCE: Duration = Duration::from_secs(1);
const CLEAR_AFTER_PAUSE: Duration = Duration::from_secs(30);

const OP_HANDSHAKE: u32 = 0;
const OP_FRAME: u32 = 1;
const OP_CLOSE: u32 = 2;
const OP_PING: u32 = 3;
const OP_PONG: u32 = 4;

trait Stream: AsyncRead + AsyncWrite + Unpin + Send {}
impl<T: AsyncRead + AsyncWrite + Unpin + Send> Stream for T {}

#[cfg(windows)]
async fn connect_ipc(id: u8) -> std::io::Result<Box<dyn Stream>> {
    let pipe = tokio::net::windows::named_pipe::ClientOptions::new().open(format!(r"\\?\pipe\discord-ipc-{id}"))?;
    Ok(Box::new(pipe))
}

#[cfg(unix)]
async fn connect_ipc(id: u8) -> std::io::Result<Box<dyn Stream>> {
    let base = ["XDG_RUNTIME_DIR", "TMPDIR", "TMP", "TEMP"]
        .iter()
        .find_map(|k| std::env::var(k).ok())
        .unwrap_or_else(|| "/tmp".into());
    let base = base.trim_end_matches('/');
    let snap = format!("{base}/snap.discord");
    let dir = if std::path::Path::new(&snap).is_dir() { snap } else { base.to_string() };
    Ok(Box::new(tokio::net::UnixStream::connect(format!("{dir}/discord-ipc-{id}")).await?))
}

async fn connect_any() -> Option<Box<dyn Stream>> {
    for id in 0..10 {
        if let Ok(stream) = connect_ipc(id).await {
            return Some(stream);
        }
    }
    None
}

async fn send(stream: &mut (impl AsyncWrite + Unpin), op: u32, payload: &Value) -> std::io::Result<()> {
    let body = payload.to_string();
    let mut frame = Vec::with_capacity(8 + body.len());
    frame.extend_from_slice(&op.to_le_bytes());
    frame.extend_from_slice(&(body.len() as u32).to_le_bytes());
    frame.extend_from_slice(body.as_bytes());
    stream.write_all(&frame).await
}

async fn read_frame(stream: &mut (impl AsyncRead + Unpin)) -> std::io::Result<(u32, Value)> {
    let mut header = [0u8; 8];
    stream.read_exact(&mut header).await?;
    let op = u32::from_le_bytes(header[..4].try_into().unwrap());
    let len = u32::from_le_bytes(header[4..].try_into().unwrap()) as usize;
    if len > 1 << 20 {
        return Err(std::io::Error::other("discord frame too large"));
    }
    let mut body = vec![0u8; len];
    stream.read_exact(&mut body).await?;
    Ok((op, serde_json::from_slice(&body).unwrap_or(Value::Null)))
}

/// Truncates to `limit` characters (Discord rejects longer) and pads to `minimum`.
fn string_limit(s: &str, limit: usize, minimum: usize) -> String {
    let count = s.chars().count();
    if count > limit {
        format!("{}...", s.chars().take(limit - 3).collect::<String>().trim())
    } else if count < minimum {
        format!("{s}{}", "\u{200B}".repeat(minimum - count))
    } else {
        s.to_string()
    }
}

fn now_ms() -> f64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as f64
}

fn activity(state: &PlayerState) -> Value {
    let Some(video) = &state.video else { return json!({ "pid": std::process::id() }) };
    let playing = state.is_playing();
    let progress = state.player.video_progress.floor();
    let mut assets = json!({ "small_text": if playing { "Playing" } else { "Paused" } });
    if let Some(thumb) = state.largest_thumbnail().filter(|t| t.url.len() <= 256) {
        assets["large_image"] = json!(thumb.url);
    }
    if let Some(album) = &video.album {
        assets["large_text"] = json!(string_limit(album, 128, 2));
    }
    if let Some(id) = &video.album_id {
        assets["large_url"] = json!(format!("https://music.youtube.com/browse/{id}"));
    }
    let watch_url = format!("https://music.youtube.com/watch?v={}", video.id);
    let mut activity = json!({
        "type": 2, // Listening
        "status_display_type": 1, // Show the state (artist) in the member list
        "details": string_limit(&video.title, 128, 2),
        "details_url": watch_url,
        "state": string_limit(&video.author, 128, 2),
        "state_url": format!("https://music.youtube.com/channel/{}", video.channel_id),
        "assets": assets,
        "instance": false,
        "buttons": [{ "label": "Listen on YouTube Music", "url": watch_url }],
    });
    if playing {
        let start = now_ms() - progress * 1000.0;
        activity["timestamps"] = json!({ "start": start as u64, "end": (start + video.duration_seconds as f64 * 1000.0) as u64 });
    }
    json!({ "pid": std::process::id(), "activity": activity })
}

fn set_activity_payload(args: Value) -> Value {
    json!({ "cmd": "SET_ACTIVITY", "args": args, "nonce": uuid::Uuid::new_v4().to_string() })
}

/// Whether a state change should refresh the activity (port of upstream heuristics).
fn needs_update(old: Option<&PlayerState>, new: &PlayerState) -> bool {
    let Some(video) = &new.video else { return true };
    if !video.metadata_filled {
        return false;
    }
    let Some(old) = old else { return true };
    let old_progress = old.player.video_progress.floor();
    let progress = new.player.video_progress.floor();
    old.player.track_state != new.player.track_state
        || old.video.as_ref().map(|v| &v.id) != Some(&video.id)
        || (progress - old_progress).abs() > 1.0
        || old_progress > progress
}

/// Runs the presence loop until aborted.
pub async fn run(app: AppHandle) {
    let ctx = app_ctx(&app);
    let client_id = ctx.settings.get().integrations.discord_client_id.trim().to_string();
    if client_id.is_empty() {
        ctx.memory.update(&app, |m| m.discord_presence_connection_failed = true);
        log::warn!("Discord presence enabled without an application ID");
        return;
    }
    let mut retries = 0;
    loop {
        let Some(stream) = connect_any().await else {
            retries += 1;
            if retries >= MAX_RETRIES {
                ctx.memory.update(&app, |m| m.discord_presence_connection_failed = true);
                return;
            }
            log::info!("Connecting to Discord attempt {retries}/{MAX_RETRIES}");
            tokio::time::sleep(RETRY_DELAY).await;
            continue;
        };
        let (mut reader, mut writer) = tokio::io::split(stream);
        if send(&mut writer, OP_HANDSHAKE, &json!({ "v": 1, "client_id": client_id })).await.is_err() {
            continue;
        }
        retries = 0;
        ctx.memory.update(&app, |m| m.discord_presence_connection_failed = false);
        log::info!("Connected to Discord");

        let mut rx = ctx.player.subscribe();
        let mut last: Option<Arc<PlayerState>> = None;
        let mut pending_update: Option<Instant> = Some(Instant::now());
        let mut clear_at: Option<Instant> = None;
        let far = || Instant::now() + Duration::from_secs(86_400);
        loop {
            tokio::select! {
                changed = rx.changed() => {
                    if changed.is_err() { return; }
                    let state = rx.borrow_and_update().clone();
                    if needs_update(last.as_deref(), &state) && pending_update.is_none() {
                        pending_update = Some(Instant::now() + DEBOUNCE);
                    }
                    clear_at = (!state.is_playing()).then(|| clear_at.unwrap_or_else(|| Instant::now() + CLEAR_AFTER_PAUSE));
                    last = Some(state);
                }
                _ = sleep_until(pending_update.unwrap_or_else(far)), if pending_update.is_some() => {
                    pending_update = None;
                    let state = ctx.player.get();
                    if send(&mut writer, OP_FRAME, &set_activity_payload(activity(&state))).await.is_err() { break; }
                }
                _ = sleep_until(clear_at.unwrap_or_else(far)), if clear_at.is_some() => {
                    clear_at = None;
                    let clear = set_activity_payload(json!({ "pid": std::process::id() }));
                    if send(&mut writer, OP_FRAME, &clear).await.is_err() { break; }
                }
                frame = read_frame(&mut reader) => match frame {
                    Ok((OP_PING, payload)) => { if send(&mut writer, OP_PONG, &payload).await.is_err() { break; } }
                    Ok((OP_CLOSE, _)) | Err(_) => break,
                    Ok(_) => {}
                }
            }
        }
        log::info!("Discord connection closed");
        tokio::time::sleep(RETRY_DELAY).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn string_limit_truncates_and_pads() {
        assert_eq!(string_limit("a", 128, 2), "a\u{200B}");
        let long = "x".repeat(200);
        let out = string_limit(&long, 128, 2);
        assert_eq!(out.chars().count(), 128);
        assert!(out.ends_with("..."));
    }

    #[tokio::test]
    async fn frames_round_trip() {
        let (mut a, mut b) = tokio::io::duplex(1024);
        send(&mut a, OP_PING, &json!({"n": 1})).await.unwrap();
        let (op, v) = read_frame(&mut b).await.unwrap();
        assert_eq!(op, OP_PING);
        assert_eq!(v["n"], 1);
    }
}
