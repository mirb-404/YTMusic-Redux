//! Player state store: normalizes raw YouTube Music data into a stable shape.
//!
//! Port of upstream `player-state-store`. The serialized form is exactly the Companion
//! Server v1 `/state` payload, so the UI, integrations and the API share one type.

use serde::Serialize;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::watch;

macro_rules! int_enum {
    ($name:ident { $($variant:ident = $value:expr),* $(,)? }) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
        #[serde(into = "i8")]
        pub enum $name { #[default] $($variant),* }
        impl From<$name> for i8 {
            fn from(v: $name) -> i8 { match v { $($name::$variant => $value),* } }
        }
    };
}

int_enum!(TrackState { Unknown = -1, Paused = 0, Playing = 1, Buffering = 2 });
int_enum!(RepeatMode { Unknown = -1, None = 0, All = 1, One = 2 });
int_enum!(LikeStatus { Unknown = -1, Dislike = 0, Indifferent = 1, Like = 2 });
int_enum!(VideoType { Unknown = -1, MusicAudio = 0, MusicVideo = 1, MusicUploaded = 2, PodcastEpisode = 3 });

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Thumbnail {
    pub url: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueueItem {
    pub thumbnails: Vec<Thumbnail>,
    pub title: String,
    pub author: String,
    pub duration: String,
    pub selected: bool,
    pub video_id: String,
    pub counterparts: Option<Vec<QueueItem>>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Queue {
    pub autoplay: bool,
    pub items: Vec<QueueItem>,
    pub automix_items: Vec<QueueItem>,
    pub is_generating: bool,
    pub is_infinite: bool,
    pub repeat_mode: RepeatMode,
    pub selected_item_index: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Player {
    pub track_state: TrackState,
    pub video_progress: f64,
    pub volume: f64,
    pub muted: bool,
    pub ad_playing: bool,
    /// Shared, so publishing a state change does not copy the whole queue.
    pub queue: Option<Arc<Queue>>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Video {
    pub author: String,
    pub channel_id: String,
    pub title: String,
    pub album: Option<String>,
    pub album_id: Option<String>,
    pub like_status: LikeStatus,
    pub thumbnails: Vec<Thumbnail>,
    pub duration_seconds: u64,
    pub id: String,
    pub is_live: bool,
    pub video_type: VideoType,
    /// True once YTM has filled the full metadata (title with features, album, art).
    pub metadata_filled: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PlayerState {
    pub player: Player,
    pub video: Option<Video>,
    /// Not reliably the playlist the song belongs to; see upstream API notes.
    pub playlist_id: Option<String>,
}

impl PlayerState {
    pub fn is_playing(&self) -> bool {
        self.player.track_state == TrackState::Playing
    }

    pub fn largest_thumbnail(&self) -> Option<&Thumbnail> {
        self.video.as_ref()?.thumbnails.iter().max_by_key(|t| t.width * t.height)
    }
}

fn text_runs(v: &Value) -> String {
    v.get("runs")
        .and_then(Value::as_array)
        .map(|runs| runs.iter().filter_map(|r| r.get("text").and_then(Value::as_str)).collect())
        .unwrap_or_default()
}

fn thumbnails(v: &Value) -> Vec<Thumbnail> {
    v.get("thumbnails")
        .and_then(Value::as_array)
        .map(|list| {
            list.iter()
                .filter_map(|t| {
                    Some(Thumbnail {
                        url: t.get("url")?.as_str()?.to_string(),
                        width: t.get("width").and_then(Value::as_u64).unwrap_or(0) as u32,
                        height: t.get("height").and_then(Value::as_u64).unwrap_or(0) as u32,
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn str_of(v: &Value, key: &str) -> String {
    v.get(key).and_then(Value::as_str).unwrap_or_default().to_string()
}

fn panel_renderer(renderer: &Value, counterparts: Option<&Value>) -> QueueItem {
    QueueItem {
        thumbnails: renderer.get("thumbnail").map(thumbnails).unwrap_or_default(),
        title: renderer.get("title").map(text_runs).unwrap_or_default(),
        author: renderer.get("shortBylineText").map(text_runs).unwrap_or_default(),
        duration: renderer.get("lengthText").map(text_runs).unwrap_or_default(),
        selected: renderer.get("selected").and_then(Value::as_bool).unwrap_or(false),
        video_id: str_of(renderer, "videoId"),
        counterparts: counterparts.and_then(Value::as_array).map(|list| {
            list.iter()
                .filter_map(|c| c.pointer("/counterpartRenderer/playlistPanelVideoRenderer"))
                .map(|r| panel_renderer(r, None))
                .collect()
        }),
    }
}

fn queue_item(item: &Value) -> Option<QueueItem> {
    if let Some(r) = item.get("playlistPanelVideoRenderer").filter(|v| !v.is_null()) {
        return Some(panel_renderer(r, None));
    }
    let wrapper = item.get("playlistPanelVideoWrapperRenderer")?;
    let primary = wrapper.pointer("/primaryRenderer/playlistPanelVideoRenderer")?;
    Some(panel_renderer(primary, wrapper.get("counterpart")))
}

fn queue_items(v: Option<&Value>) -> Vec<QueueItem> {
    v.and_then(Value::as_array).map(|l| l.iter().filter_map(queue_item).collect()).unwrap_or_default()
}

fn repeat_mode(s: Option<&str>) -> RepeatMode {
    match s {
        Some("NONE") => RepeatMode::None,
        Some("ALL") => RepeatMode::All,
        Some("ONE") => RepeatMode::One,
        _ => RepeatMode::Unknown,
    }
}

pub fn like_status(s: Option<&str>) -> LikeStatus {
    match s {
        Some("DISLIKE") => LikeStatus::Dislike,
        Some("INDIFFERENT") => LikeStatus::Indifferent,
        Some("LIKE") => LikeStatus::Like,
        _ => LikeStatus::Unknown,
    }
}

fn video_type(s: Option<&str>) -> VideoType {
    match s {
        Some("MUSIC_VIDEO_TYPE_ATV") => VideoType::MusicAudio,
        Some("MUSIC_VIDEO_TYPE_OMV") | Some("MUSIC_VIDEO_TYPE_UGC") => VideoType::MusicVideo,
        Some("MUSIC_VIDEO_TYPE_PRIVATELY_OWNED_TRACK") => VideoType::MusicUploaded,
        Some("MUSIC_VIDEO_TYPE_PODCAST_EPISODE") => VideoType::PodcastEpisode,
        _ => VideoType::Unknown,
    }
}

/// Owns the current state and broadcasts every change to subscribers.
pub struct PlayerStore {
    tx: watch::Sender<Arc<PlayerState>>,
}

impl PlayerStore {
    pub fn new() -> Self {
        Self { tx: watch::channel(Arc::new(PlayerState::default())).0 }
    }

    pub fn get(&self) -> Arc<PlayerState> {
        self.tx.borrow().clone()
    }

    pub fn subscribe(&self) -> watch::Receiver<Arc<PlayerState>> {
        self.tx.subscribe()
    }

    /// Publishes only real changes. YTM reports the same values over and over, and every
    /// publish wakes each subscriber and re-sends the state to the UI webviews.
    fn modify(&self, f: impl FnOnce(&mut PlayerState)) {
        self.tx.send_if_modified(|state| {
            let mut next = (**state).clone();
            f(&mut next);
            if next == **state {
                return false;
            }
            *state = Arc::new(next);
            true
        });
    }

    /// YTM reports progress several times a second. Subscribers need whole seconds and
    /// seeks, and the UI interpolates in between, so only those are published.
    pub fn update_progress(&self, progress: f64) {
        self.tx.send_if_modified(|state| {
            let old = state.player.video_progress;
            if old.floor() == progress.floor() && (progress - old).abs() < 1.0 {
                return false;
            }
            Arc::make_mut(state).player.video_progress = progress;
            true
        });
    }

    /// `state` is the raw YTM player state (-1 unstarted, 0 ended, 1 playing, 2 paused, 3 buffering, 5 cued).
    pub fn update_track_state(&self, state: i64) {
        self.modify(|s| {
            s.player.track_state = match state {
                1 => TrackState::Playing,
                2 => TrackState::Paused,
                3 => TrackState::Buffering,
                _ => TrackState::Unknown,
            }
        });
    }

    pub fn update_video(&self, details: &Value, playlist_id: Option<String>, album: Option<&Value>, like: Option<&str>, full: bool) {
        let video = Video {
            author: str_of(details, "author"),
            channel_id: str_of(details, "channelId"),
            title: str_of(details, "title"),
            album: album.and_then(|a| a.get("text")).and_then(Value::as_str).map(str::to_string),
            album_id: album.and_then(|a| a.get("id")).and_then(Value::as_str).map(str::to_string),
            like_status: like_status(like),
            thumbnails: details.get("thumbnail").map(thumbnails).unwrap_or_default(),
            duration_seconds: details
                .get("lengthSeconds")
                .and_then(|v| v.as_str().and_then(|s| s.parse().ok()).or_else(|| v.as_u64()))
                .unwrap_or(0),
            id: str_of(details, "videoId"),
            is_live: details.get("isLive").and_then(Value::as_bool).unwrap_or(false),
            video_type: video_type(details.get("musicVideoType").and_then(Value::as_str)),
            metadata_filled: full,
        };
        self.modify(|s| {
            s.video = Some(video);
            s.playlist_id = playlist_id;
        });
    }

    pub fn update_from_store(&self, queue: Option<&Value>, like: Option<&str>, volume: Option<f64>, muted: bool, ad_playing: bool) {
        let queue = queue.filter(|q| !q.is_null()).map(|q| {
            let items = queue_items(q.get("items"));
            // YTM's own selectedItemIndex is unreliable, so it is derived from the items.
            let selected_item_index = items.iter().position(|i| i.selected).map_or(-1, |i| i as i64);
            Queue {
                automix_items: queue_items(q.get("automixItems")),
                autoplay: q.get("autoplay").and_then(Value::as_bool).unwrap_or(false),
                is_generating: q.get("isGenerating").and_then(Value::as_bool).unwrap_or(false),
                is_infinite: q.get("isInfinite").and_then(Value::as_bool).unwrap_or(false),
                repeat_mode: repeat_mode(q.get("repeatMode").and_then(Value::as_str)),
                items,
                selected_item_index,
            }
        });
        // An unchanged queue keeps its allocation, so the comparison in `modify` stays cheap.
        let current = self.get();
        let queue = match (&current.player.queue, queue) {
            (Some(old), Some(new)) if **old == new => Some(old.clone()),
            (_, new) => new.map(Arc::new),
        };
        self.modify(|s| {
            s.player.queue = queue;
            if let Some(video) = s.video.as_mut() {
                video.like_status = like_status(like);
            }
            s.player.ad_playing = ad_playing;
            s.player.muted = muted;
            if let Some(v) = volume.filter(|v| *v >= 0.0) {
                s.player.volume = v;
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn maps_video_and_queue_to_api_shape() {
        let store = PlayerStore::new();
        store.update_video(
            &json!({"author":"A","channelId":"c","title":"T","lengthSeconds":"215","videoId":"v1",
                    "musicVideoType":"MUSIC_VIDEO_TYPE_ATV","thumbnail":{"thumbnails":[{"url":"u","width":60,"height":60}]}}),
            Some("PL".into()),
            Some(&json!({"id":"MPRE","text":"Album"})),
            Some("LIKE"),
            true,
        );
        store.update_from_store(
            Some(&json!({"items":[
                {"playlistPanelVideoRenderer":{"title":{"runs":[{"text":"x"}]},"videoId":"a","selected":false}},
                {"playlistPanelVideoWrapperRenderer":{"primaryRenderer":{"playlistPanelVideoRenderer":{"videoId":"b","selected":true}},
                  "counterpart":[{"counterpartRenderer":{"playlistPanelVideoRenderer":{"videoId":"b2"}}}]}}
            ],"automixItems":[],"repeatMode":"ALL","autoplay":true})),
            Some("LIKE"),
            Some(40.0),
            false,
            false,
        );
        store.update_track_state(1);
        let v = serde_json::to_value(&*store.get()).unwrap();
        assert_eq!(v["video"]["durationSeconds"], 215);
        assert_eq!(v["video"]["likeStatus"], 2);
        assert_eq!(v["video"]["videoType"], 0);
        assert_eq!(v["video"]["album"], "Album");
        assert_eq!(v["video"]["metadataFilled"], true);
        assert_eq!(v["player"]["trackState"], 1);
        assert_eq!(v["player"]["queue"]["repeatMode"], 1);
        assert_eq!(v["player"]["queue"]["selectedItemIndex"], 1);
        assert_eq!(v["player"]["queue"]["items"][1]["counterparts"][0]["videoId"], "b2");
        assert_eq!(v["playlistId"], "PL");
    }

    #[test]
    fn negative_volume_is_ignored() {
        let store = PlayerStore::new();
        store.update_from_store(None, None, Some(55.0), false, false);
        store.update_from_store(None, None, Some(-1.0), true, false);
        let s = store.get();
        assert_eq!(s.player.volume, 55.0);
        assert!(s.player.muted);
    }
}
