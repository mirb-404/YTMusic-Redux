//! Last.fm scrobbling (port of upstream `last-fm` integration).

use crate::app_ctx;
use crate::player::{PlayerState, Video};
use crate::state_store::secrets;
use md5::{Digest, Md5};
use serde_json::Value;
use std::collections::BTreeMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::AppHandle;
use tauri_plugin_opener::OpenerExt;
use tokio::time::{sleep_until, Instant};

const API: &str = "https://ws.audioscrobbler.com/2.0/";
const SESSION_SECRET: &str = "lastfm-session";

/// Signs a request per https://www.last.fm/api/authspec#_8-signing-calls.
fn api_sig(params: &BTreeMap<&str, String>, secret: &str) -> String {
    let mut data: String = params
        .iter()
        .filter(|(k, v)| **k != "format" && **k != "callback" && !v.is_empty())
        .map(|(k, v)| format!("{k}{v}"))
        .collect();
    data.push_str(secret);
    hex::encode(Md5::digest(data.as_bytes()))
}

struct Client {
    http: reqwest::Client,
    api_key: String,
    secret: String,
}

impl Client {
    fn signed(&self, mut params: BTreeMap<&'static str, String>) -> BTreeMap<&'static str, String> {
        params.insert("api_key", self.api_key.clone());
        params.insert("format", "json".into());
        let sig = api_sig(&params, &self.secret);
        params.insert("api_sig", sig);
        params.retain(|_, v| !v.is_empty());
        params
    }

    async fn get(&self, params: BTreeMap<&'static str, String>) -> Result<Value, reqwest::Error> {
        self.http.get(API).query(&self.signed(params)).send().await?.json().await
    }

    async fn post(&self, params: BTreeMap<&'static str, String>) -> Result<Value, reqwest::Error> {
        self.http.post(API).form(&self.signed(params)).send().await?.json().await
    }
}

fn set_status(app: &AppHandle, status: Option<&str>) {
    let status = status.map(str::to_string);
    app_ctx(app).memory.update(app, |m| m.lastfm_status = status);
}

/// Obtains a session key, sending the user to Last.fm to approve the app when needed.
async fn ensure_session(app: &AppHandle, client: &Client) -> Option<String> {
    if let Some(key) = secrets::get(SESSION_SECRET) {
        return Some(key);
    }
    let ctx = app_ctx(app);
    if let Some(token) = ctx.state.read(|s| s.lastfm_token.clone()) {
        let params = BTreeMap::from([("method", "auth.getSession".to_string()), ("token", token)]);
        if let Ok(json) = client.get(params).await {
            if let Some(key) = json.pointer("/session/key").and_then(Value::as_str) {
                secrets::set(SESSION_SECRET, Some(key));
                ctx.state.update(|s| s.lastfm_token = None);
                set_status(app, None);
                return Some(key.to_string());
            }
        }
    }
    // No usable token: request one and open the approval page.
    let json = client.get(BTreeMap::from([("method", "auth.gettoken".to_string())])).await.ok()?;
    let token = json.get("token").and_then(Value::as_str)?.to_string();
    ctx.state.update(|s| s.lastfm_token = Some(token.clone()));
    let url = format!(
        "https://www.last.fm/api/auth/?api_key={}&token={}",
        urlencode(&client.api_key),
        urlencode(&token)
    );
    let _ = app.opener().open_url(url, None::<&str>);
    set_status(app, Some("Approve the app on Last.fm, then play a song to finish connecting."));
    None
}

fn urlencode(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

async fn send_track(app: &AppHandle, client: &Client, session: &str, method: &str, video: &Video, timestamp: Option<u64>) {
    let mut params = BTreeMap::from([
        ("method", method.to_string()),
        ("artist", video.author.clone()),
        ("track", video.title.clone()),
        ("album", video.album.clone().unwrap_or_default()),
        ("duration", video.duration_seconds.to_string()),
        ("sk", session.to_string()),
    ]);
    if let Some(ts) = timestamp {
        params.insert("timestamp", ts.to_string());
    }
    match client.post(params).await {
        // Error 9: invalid session key (https://www.last.fm/api/show/track.scrobble#errors)
        Ok(json) if json.get("error").and_then(Value::as_i64) == Some(9) => {
            secrets::set(SESSION_SECRET, None);
            ensure_session(app, client).await;
        }
        Ok(json) if json.get("error").is_some() => log::warn!("Last.fm {method} failed: {}", json["message"]),
        Ok(_) => {}
        Err(e) => log::warn!("Last.fm {method} request failed: {e}"),
    }
}

/// Every video ID of the current song (audio and video counterparts).
fn song_video_ids(state: &PlayerState) -> Vec<String> {
    let Some(queue) = &state.player.queue else { return Vec::new() };
    let Some(item) = usize::try_from(queue.selected_item_index).ok().and_then(|i| queue.items.get(i)) else { return Vec::new() };
    let mut ids: Vec<String> = item.counterparts.iter().flatten().map(|c| c.video_id.clone()).collect();
    ids.push(item.video_id.clone());
    ids
}

/// Time until scrobble: the chosen percentage of the track, at most 4 minutes.
fn scrobble_delay(duration_seconds: u64, percent: u32) -> Duration {
    let secs = ((duration_seconds as f64) * f64::from(percent) / 100.0).round() as u64;
    Duration::from_secs(secs.min(240))
}

pub async fn run(app: AppHandle) {
    let ctx = app_ctx(&app);
    let config = ctx.settings.get().lastfm.clone();
    if config.api_key.is_empty() || config.secret.is_empty() {
        set_status(&app, Some("Enter your Last.fm API key and secret to enable scrobbling."));
        return;
    }
    let client = Client { http: reqwest::Client::new(), api_key: config.api_key, secret: config.secret };
    set_status(&app, None);
    let mut session = ensure_session(&app, &client).await;

    let mut rx = ctx.player.subscribe();
    let mut current_ids: Vec<String> = Vec::new();
    let mut scrobble: Option<(Instant, Video, u64)> = None;
    let far = || Instant::now() + Duration::from_secs(86_400);
    loop {
        tokio::select! {
            changed = rx.changed() => {
                if changed.is_err() { return; }
                let state = rx.borrow_and_update().clone();
                let Some(video) = state.video.clone().filter(|_| state.is_playing()) else { continue };
                let has_queue = state.player.queue.as_ref().is_some_and(|q| !q.items.is_empty());
                if has_queue && current_ids.contains(&video.id) {
                    continue;
                }
                current_ids = song_video_ids(&state);
                if session.is_none() {
                    session = ensure_session(&app, &client).await;
                    if session.is_none() { continue; }
                }
                scrobble = None;
                // Tracks must be longer than 30 seconds (https://www.last.fm/api/scrobbling).
                if video.duration_seconds < 30 { continue; }
                send_track(&app, &client, session.as_deref().unwrap(), "track.updateNowPlaying", &video, None).await;
                let started = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
                let percent = app_ctx(&app).settings.get().lastfm.scrobble_percent;
                scrobble = Some((Instant::now() + scrobble_delay(video.duration_seconds, percent), video, started));
            }
            _ = sleep_until(scrobble.as_ref().map_or_else(far, |s| s.0)), if scrobble.is_some() => {
                if let (Some((_, video, started)), Some(sk)) = (scrobble.take(), session.as_deref()) {
                    send_track(&app, &client, sk, "track.scrobble", &video, Some(started)).await;
                }
            }
        }
    }
}

/// Forgets the stored session so the next enable re-authenticates.
pub fn sign_out(app: &AppHandle) {
    secrets::set(SESSION_SECRET, None);
    app_ctx(app).state.update(|s| s.lastfm_token = None);
}

pub fn is_signed_in() -> bool {
    secrets::get(SESSION_SECRET).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signature_sorts_and_skips_format() {
        let params = BTreeMap::from([("method", "auth.gettoken".to_string()), ("api_key", "k".into()), ("format", "json".into())]);
        let expected = hex::encode(Md5::digest(b"api_keykmethodauth.gettokens3cr3t"));
        assert_eq!(api_sig(&params, "s3cr3t"), expected);
    }

    #[test]
    fn scrobble_delay_caps_at_four_minutes() {
        assert_eq!(scrobble_delay(200, 50), Duration::from_secs(100));
        assert_eq!(scrobble_delay(3600, 50), Duration::from_secs(240));
    }
}
