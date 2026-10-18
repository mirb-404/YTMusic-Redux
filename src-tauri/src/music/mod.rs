//! Native browsing: the app draws YouTube Music's pages itself from YTM's web client API
//! (`music.youtube.com/youtubei/v1/*`, what music.youtube.com itself calls), signed in
//! with the YTM view's own session cookies. The YTM page stays loaded as the playback
//! engine: playing an item hands its watch endpoint to that page.

pub mod parse;

use crate::ytm;
use serde_json::{json, Value};
use sha1::{Digest, Sha1};
use std::sync::{LazyLock, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::AppHandle;

const ORIGIN: &str = "https://music.youtube.com";
/// Used until the YTM page reports the client it runs (`ytm_innertube`).
const FALLBACK_CLIENT_VERSION: &str = "1.20261013.15.00";
const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/141.0.0.0 Safari/537.36 Edg/141.0.0.0";

type Result<T> = std::result::Result<T, String>;

/// The client fields of YTM's own request context (version, language, region, visitor).
static CLIENT: Mutex<Option<Value>> = Mutex::new(None);

static HTTP: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .user_agent(UA)
        .gzip(true)
        .brotli(true)
        .pool_idle_timeout(std::time::Duration::from_secs(90))
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .expect("http client")
});

/// Stores the client YTM's page runs, so requests match what YTM itself sends.
pub fn set_client(client: &Value) {
    let mut kept = serde_json::Map::new();
    for key in ["clientName", "clientVersion", "hl", "gl", "visitorData", "timeZone", "utcOffsetMinutes"] {
        if let Some(v) = client.get(key) {
            kept.insert(key.into(), v.clone());
        }
    }
    if kept.get("clientName").and_then(Value::as_str) == Some("WEB_REMIX") {
        *CLIENT.lock().unwrap() = Some(Value::Object(kept));
    }
}

fn client() -> Value {
    CLIENT.lock().unwrap().clone().unwrap_or_else(|| json!({ "clientName": "WEB_REMIX", "clientVersion": FALLBACK_CLIENT_VERSION, "hl": "en" }))
}

/// The YTM view's cookies for music.youtube.com, HttpOnly ones included. WebView2 cookie
/// access must not run on the main thread.
async fn cookies(app: &AppHandle) -> Result<Vec<(String, String)>> {
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let view = ytm::webview(&app).ok_or("YouTube Music is not loaded")?;
        let list = view.cookies_for_url(ORIGIN.parse().unwrap()).map_err(|e| e.to_string())?;
        Ok(list.iter().map(|c| (c.name().to_string(), c.value().to_string())).collect())
    })
    .await
    .map_err(|e| e.to_string())?
}

fn cookie<'a>(cookies: &'a [(String, String)], name: &str) -> Option<&'a str> {
    cookies.iter().find(|(n, _)| n == name).map(|(_, v)| v.as_str())
}

/// The `Authorization` header YTM sends for signed-in calls.
fn sapisid_hash(cookies: &[(String, String)]) -> Option<String> {
    let ts = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs();
    let hash = |sid: &str| format!("{ts}_{}", hex::encode(Sha1::digest(format!("{ts} {sid} {ORIGIN}").as_bytes())));
    let mut parts = Vec::new();
    if let Some(s) = cookie(cookies, "SAPISID").or(cookie(cookies, "__Secure-3PAPISID")) {
        parts.push(format!("SAPISIDHASH {}", hash(s)));
    }
    if let Some(s) = cookie(cookies, "__Secure-1PAPISID") {
        parts.push(format!("SAPISID1PHASH {}", hash(s)));
    }
    if let Some(s) = cookie(cookies, "__Secure-3PAPISID") {
        parts.push(format!("SAPISID3PHASH {}", hash(s)));
    }
    (!parts.is_empty()).then(|| parts.join(" "))
}

async fn call(app: &AppHandle, endpoint: &str, query: &str, mut body: Value) -> Result<Value> {
    let cookies = cookies(app).await?;
    let client = client();
    body["context"] = json!({ "client": client, "user": {} });
    let version = client.get("clientVersion").and_then(Value::as_str).unwrap_or(FALLBACK_CLIENT_VERSION).to_string();
    let mut request = HTTP
        .post(format!("{ORIGIN}/youtubei/v1/{endpoint}?prettyPrint=false{query}"))
        .header("Content-Type", "application/json")
        .header("Origin", ORIGIN)
        .header("Referer", format!("{ORIGIN}/"))
        .header("X-Origin", ORIGIN)
        .header("X-Goog-AuthUser", "0")
        .header("X-Youtube-Client-Name", "67")
        .header("X-Youtube-Client-Version", version)
        .header("Cookie", cookies.iter().map(|(n, v)| format!("{n}={v}")).collect::<Vec<_>>().join("; "));
    if let Some(auth) = sapisid_hash(&cookies) {
        request = request.header("Authorization", auth);
    }
    let response = request.json(&body).send().await.map_err(|e| format!("YouTube Music could not be reached: {e}"))?;
    let status = response.status();
    let value: Value = response.json().await.map_err(|e| e.to_string())?;
    if !status.is_success() {
        let message = value.pointer("/error/message").and_then(Value::as_str).unwrap_or("request failed");
        return Err(format!("YouTube Music: {message} ({status})"));
    }
    Ok(value)
}

/// Accepts only the endpoint shapes the UI may follow, from a parsed page.
fn browse_target(endpoint: &Value) -> Result<(String, Option<String>)> {
    let e = endpoint.get("browseEndpoint").ok_or("not a browse endpoint")?;
    let id = e.get("browseId").and_then(Value::as_str).filter(|s| !s.is_empty()).ok_or("browse endpoint without an id")?;
    Ok((id.to_string(), e.get("params").and_then(Value::as_str).map(str::to_string)))
}

#[tauri::command]
pub async fn music_browse(app: AppHandle, endpoint: Value) -> Result<parse::Page> {
    let (browse_id, params) = browse_target(&endpoint)?;
    let mut body = json!({ "browseId": browse_id });
    if let Some(p) = params {
        body["params"] = json!(p);
    }
    Ok(parse::page(&call(&app, "browse", "", body).await?))
}

#[tauri::command]
pub async fn music_search(app: AppHandle, query: String, params: Option<String>) -> Result<parse::Page> {
    let mut body = json!({ "query": query });
    if let Some(p) = params {
        body["params"] = json!(p);
    }
    Ok(parse::page(&call(&app, "search", "", body).await?))
}

#[tauri::command]
pub async fn music_continue(app: AppHandle, continuation: parse::Continuation) -> Result<parse::Page> {
    let response = if continuation.query {
        let token = urlencode(&continuation.token);
        call(&app, "browse", &format!("&ctoken={token}&continuation={token}&type=next"), json!({})).await?
    } else {
        call(&app, "browse", "", json!({ "continuation": continuation.token })).await?
    };
    Ok(parse::continuation_page(&response))
}

#[tauri::command]
pub async fn music_suggestions(app: AppHandle, input: String) -> Result<parse::Suggestions> {
    Ok(parse::suggestions(&call(&app, "music/get_search_suggestions", "", json!({ "input": input })).await?))
}

#[tauri::command]
pub async fn music_guide(app: AppHandle) -> Result<Vec<parse::GuideEntry>> {
    Ok(parse::guide(&call(&app, "guide", "", json!({})).await?))
}

#[tauri::command]
pub async fn music_signed_in(app: AppHandle) -> Result<bool> {
    let cookies = cookies(&app).await?;
    Ok(cookie(&cookies, "SAPISID").or(cookie(&cookies, "__Secure-3PAPISID")).is_some())
}

#[tauri::command]
pub fn music_sign_in(app: AppHandle) {
    ytm::sign_in(&app);
}

/// Sends one of the requests an item's menu offers: like, library feedback, or an edit of
/// the user's own playlist. The endpoint is trimmed again here, so only those go out.
#[tauri::command]
pub async fn music_service(app: AppHandle, endpoint: Value) -> Result<()> {
    let endpoint = parse::service(Some(&endpoint)).ok_or("not a supported request")?;
    if let Some(l) = endpoint.get("likeEndpoint") {
        let path = match l.get("status").and_then(Value::as_str) {
            Some("LIKE") => "like/like",
            Some("DISLIKE") => "like/dislike",
            Some("INDIFFERENT") => "like/removelike",
            _ => return Err("unknown like status".into()),
        };
        let target = l.get("target").cloned().ok_or("like without a target")?;
        call(&app, path, "", json!({ "target": target })).await?;
    } else if let Some(f) = endpoint.get("feedbackEndpoint") {
        call(&app, "feedback", "", json!({ "feedbackTokens": [f["feedbackToken"]] })).await?;
    } else if let Some(p) = endpoint.get("playlistEditEndpoint") {
        call(&app, "browse/edit_playlist", "", json!({ "playlistId": p["playlistId"], "actions": p["actions"] })).await?;
    }
    Ok(())
}

/// Likes, dislikes or clears a track's rating.
#[tauri::command]
pub async fn music_rate(app: AppHandle, video_id: String, status: String) -> Result<()> {
    music_service(app, json!({ "likeEndpoint": { "status": status, "target": { "videoId": video_id } } })).await
}

fn queue_body(video_id: Option<String>, playlist_id: Option<String>) -> Result<Value> {
    match (video_id, playlist_id) {
        (Some(v), _) => Ok(json!({ "videoIds": [v] })),
        (None, Some(p)) => Ok(json!({ "playlistId": p })),
        (None, None) => Err("nothing to add".into()),
    }
}

/// Play next, or add to the end of the queue. The queue lives in the YTM page, which takes
/// the items YTM's own queue endpoint returns for the track or list.
#[tauri::command]
pub async fn music_queue(app: AppHandle, next: bool, video_id: Option<String>, playlist_id: Option<String>) -> Result<()> {
    let nothing_playing = crate::app_ctx(&app).player.get().video.is_none();
    if nothing_playing {
        let endpoint = match (&video_id, &playlist_id) {
            (Some(v), _) => json!({ "watchEndpoint": { "videoId": v } }),
            (None, Some(p)) => json!({ "watchPlaylistEndpoint": { "playlistId": p } }),
            _ => return Err("nothing to add".into()),
        };
        return music_play(app, endpoint);
    }
    let response = call(&app, "music/get_queue", "", queue_body(video_id, playlist_id)?).await?;
    let items: Vec<Value> = response.get("queueDatas").and_then(Value::as_array).map(|q| q.iter().filter_map(|d| d.get("content").cloned()).collect()).unwrap_or_default();
    if items.is_empty() {
        return Err("YouTube Music returned nothing to queue".into());
    }
    ytm::call(&app, "queueAdd", &[Value::Array(items), json!(next)]);
    Ok(())
}

fn queue_len(app: &AppHandle) -> usize {
    crate::app_ctx(app).player.get().player.queue.as_ref().map_or(0, |q| q.items.len())
}

#[tauri::command]
pub fn music_queue_remove(app: AppHandle, index: usize) -> Result<()> {
    if index >= queue_len(&app) {
        return Err("no such queue entry".into());
    }
    ytm::call(&app, "queueRemove", &[json!(index)]);
    Ok(())
}

#[tauri::command]
pub fn music_queue_move(app: AppHandle, from: usize, to: usize) -> Result<()> {
    let len = queue_len(&app);
    if from >= len || to >= len {
        return Err("no such queue entry".into());
    }
    ytm::call(&app, "queueMove", &[json!(from), json!(to)]);
    Ok(())
}

#[tauri::command]
pub async fn music_playlist_targets(app: AppHandle, video_id: Option<String>, playlist_id: Option<String>) -> Result<Vec<parse::PlaylistTarget>> {
    let body = queue_body(video_id, playlist_id)?;
    Ok(parse::playlist_targets(&call(&app, "playlist/get_add_to_playlist", "", body).await?))
}

/// Saves a track, or every track of a list, to one of the user's playlists.
#[tauri::command]
pub async fn music_add_to_playlist(app: AppHandle, playlist_id: String, video_id: Option<String>, source_playlist_id: Option<String>) -> Result<()> {
    let action = match (video_id, source_playlist_id) {
        (Some(v), _) => json!({ "action": "ACTION_ADD_VIDEO", "addedVideoId": v, "dedupeOption": "DEDUPE_OPTION_SKIP" }),
        (None, Some(p)) => json!({ "action": "ACTION_ADD_PLAYLIST", "addedFullListId": p, "dedupeOption": "DEDUPE_OPTION_SKIP" }),
        (None, None) => return Err("nothing to add".into()),
    };
    call(&app, "browse/edit_playlist", "", json!({ "playlistId": playlist_id, "actions": [action] })).await?;
    Ok(())
}

/// Creates a playlist, optionally with a first track or a copy of a list; returns its id.
#[tauri::command]
pub async fn music_create_playlist(
    app: AppHandle,
    title: String,
    description: String,
    privacy: String,
    video_id: Option<String>,
    source_playlist_id: Option<String>,
) -> Result<String> {
    let title = title.trim();
    if title.is_empty() || title.chars().count() > 150 {
        return Err("a playlist title needs 1 to 150 characters".into());
    }
    if !matches!(privacy.as_str(), "PUBLIC" | "UNLISTED" | "PRIVATE") {
        return Err("unknown privacy".into());
    }
    let mut body = json!({ "title": title, "description": description, "privacyStatus": privacy });
    if let Some(v) = video_id {
        body["videoIds"] = json!([v]);
    }
    if let Some(p) = source_playlist_id {
        body["sourcePlaylistId"] = json!(p);
    }
    let response = call(&app, "playlist/create", "", body).await?;
    response.get("playlistId").and_then(Value::as_str).map(str::to_string).ok_or_else(|| "YouTube Music did not create the playlist".into())
}

/// Plays a song, video, album or playlist in the YTM page, as clicking it there would.
#[tauri::command]
pub fn music_play(app: AppHandle, endpoint: Value) -> Result<()> {
    let endpoint = watch_endpoint(&endpoint)?;
    ytm::call(&app, "playQuietly", &[endpoint]);
    Ok(())
}

/// Rebuilds a trimmed watch endpoint into the shape YTM's player expects.
fn watch_endpoint(endpoint: &Value) -> Result<Value> {
    if let Some(w) = endpoint.get("watchEndpoint") {
        let mut out = json!({});
        for field in ["videoId", "playlistId", "params", "index"] {
            if let Some(v) = w.get(field) {
                out[field] = v.clone();
            }
        }
        if out.get("videoId").is_none() && out.get("playlistId").is_none() {
            return Err("watch endpoint without a track".into());
        }
        if let Some(t) = w.get("musicVideoType") {
            out["watchEndpointMusicSupportedConfigs"] = json!({ "watchEndpointMusicConfig": { "musicVideoType": t } });
        }
        return Ok(json!({ "watchEndpoint": out }));
    }
    if let Some(w) = endpoint.get("watchPlaylistEndpoint") {
        let id = w.get("playlistId").and_then(Value::as_str).ok_or("playlist endpoint without an id")?;
        let mut out = json!({ "playlistId": id });
        if let Some(p) = w.get("params") {
            out["params"] = p.clone();
        }
        return Ok(json!({ "watchPlaylistEndpoint": out }));
    }
    Err("not a playable endpoint".into())
}

fn urlencode(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn watch_endpoints_are_rebuilt() {
        let e = watch_endpoint(&json!({ "watchEndpoint": { "videoId": "abc", "playlistId": "PL1", "musicVideoType": "MUSIC_VIDEO_TYPE_ATV" } })).unwrap();
        assert_eq!(
            e,
            json!({ "watchEndpoint": { "videoId": "abc", "playlistId": "PL1", "watchEndpointMusicSupportedConfigs": { "watchEndpointMusicConfig": { "musicVideoType": "MUSIC_VIDEO_TYPE_ATV" } } } })
        );
        assert!(watch_endpoint(&json!({ "browseEndpoint": { "browseId": "x" } })).is_err());
        assert!(watch_endpoint(&json!({ "watchEndpoint": {} })).is_err());
    }

    #[test]
    fn browse_targets_need_an_id() {
        assert_eq!(browse_target(&json!({ "browseEndpoint": { "browseId": "FEmusic_home" } })).unwrap(), ("FEmusic_home".into(), None));
        assert!(browse_target(&json!({ "watchEndpoint": { "videoId": "x" } })).is_err());
    }

    #[test]
    fn hash_matches_the_web_client_format() {
        let h = sapisid_hash(&[("SAPISID".into(), "abc".into())]).unwrap();
        let (ts, digest) = h.strip_prefix("SAPISIDHASH ").unwrap().split_once('_').unwrap();
        assert_eq!(digest, hex::encode(Sha1::digest(format!("{ts} abc {ORIGIN}").as_bytes())));
    }
}
