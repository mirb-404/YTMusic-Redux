//! Lyrics from LRCLIB (https://lrclib.net), a free community lyrics database that needs no
//! API key. Time-coded (synced) lyrics drive the now playing view and lyric captions.

use crate::player::Video;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use std::time::Duration;

const SEARCH_URL: &str = "https://lrclib.net/api/search";
/// Results whose length differs more than this from the track are another recording.
const DURATION_TOLERANCE: f64 = 4.0;
const CACHE_LIMIT: usize = 200;

static CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        // LRCLIB asks clients to identify themselves.
        .user_agent(concat!("YTMusicRedux/", env!("CARGO_PKG_VERSION")))
        .timeout(Duration::from_secs(10))
        .build()
        .expect("HTTP client")
});
static TIMESTAMP: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\[(\d+):(\d+(?:\.\d+)?)\]").unwrap());
/// Video decorations such as "(Official Music Video)" or "[Lyrics]".
static NOISE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\s*[(\[][^)\]]*\b(official|video|lyrics?|audio|visuali[sz]er|mv|hd|4k|remaster(ed)?)\b[^)\]]*[)\]]").unwrap()
});
static FEATURING: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)\s*[(\[]?\s*\b(feat\.?|ft\.|featuring)\s.*$").unwrap());

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Line {
    /// Seconds from the start of the track.
    pub time: f64,
    pub text: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Lyrics {
    pub video_id: String,
    pub synced: Option<Vec<Line>>,
    pub plain: Option<String>,
    pub instrumental: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Record {
    duration: Option<f64>,
    instrumental: Option<bool>,
    plain_lyrics: Option<String>,
    synced_lyrics: Option<String>,
}

/// Lyrics per video ID. A cached `None` is a confirmed miss; network errors are not cached.
#[derive(Default)]
pub struct Cache(Mutex<HashMap<String, Option<Lyrics>>>);

impl Cache {
    pub async fn get(&self, video: &Video) -> Result<Option<Lyrics>, String> {
        let cached = self.0.lock().unwrap().get(&video.id).cloned();
        if let Some(hit) = cached {
            return Ok(hit);
        }
        let lyrics = fetch(video).await.map_err(|e| {
            log::warn!("Lyrics lookup failed: {e}");
            "Couldn't reach the lyrics service. Check your connection and try again.".to_string()
        })?;
        let mut map = self.0.lock().unwrap();
        if map.len() >= CACHE_LIMIT {
            map.clear();
        }
        map.insert(video.id.clone(), lyrics.clone());
        Ok(lyrics)
    }
}

async fn search(query: &[(&str, &str)]) -> reqwest::Result<Vec<Record>> {
    CLIENT.get(SEARCH_URL).query(query).send().await?.error_for_status()?.json().await
}

async fn fetch(video: &Video) -> reqwest::Result<Option<Lyrics>> {
    let (track, artist) = search_terms(&video.title, &video.author);
    let mut records = search(&[("track_name", &track), ("artist_name", &artist)]).await?;
    if records.is_empty() {
        // Free-text search is a best-effort fallback; LRCLIB fails some of these queries.
        records = search(&[("q", &format!("{track} {artist}"))]).await.unwrap_or_else(|e| {
            log::warn!("LRCLIB fallback search failed: {e}");
            Vec::new()
        });
    }
    let Some(record) = best_match(records, video.duration_seconds as f64) else { return Ok(None) };
    let synced = record.synced_lyrics.as_deref().map(parse_lrc).filter(|l| !l.is_empty());
    Ok(Some(Lyrics {
        video_id: video.id.clone(),
        synced,
        plain: record.plain_lyrics.filter(|p| !p.trim().is_empty()),
        instrumental: record.instrumental.unwrap_or(false),
    }))
}

/// First result (LRCLIB orders by relevance) of matching length, preferring synced lyrics.
fn best_match(records: Vec<Record>, duration: f64) -> Option<Record> {
    records
        .into_iter()
        .filter(|r| duration <= 0.0 || r.duration.is_none_or(|d| (d - duration).abs() <= DURATION_TOLERANCE))
        .min_by_key(|r| (r.synced_lyrics.is_none(), r.plain_lyrics.is_none() && r.instrumental != Some(true)))
}

/// Track name and primary artist without video decorations. Uploaded videos are often
/// titled "Artist - Title" under a channel name, so that form is split when it matches.
fn search_terms(title: &str, author: &str) -> (String, String) {
    let clean_artist = |a: &str| a.trim().trim_end_matches(" - Topic").trim_end_matches("VEVO").trim().to_string();
    let mut artist = clean_artist(author);
    let mut track = title.to_string();
    if let Some((left, right)) = title.split_once(" - ") {
        let (l, a) = (clean_artist(left).to_lowercase(), artist.to_lowercase());
        if !l.is_empty() && (a.is_empty() || l.contains(&a) || a.contains(&l)) {
            artist = clean_artist(left);
            track = right.to_string();
        }
    }
    let track = FEATURING.replace(&NOISE.replace_all(&track, ""), "").trim().to_string();
    let artist = artist.split([',', '&']).next().unwrap_or_default().trim().to_string();
    (track, artist)
}

/// Parses LRC text: `[mm:ss.xx] line`, with any number of leading timestamps per line.
pub fn parse_lrc(text: &str) -> Vec<Line> {
    let mut lines = Vec::new();
    for raw in text.lines() {
        let mut rest = raw.trim_start();
        let mut times = Vec::new();
        while let Some(c) = TIMESTAMP.captures(rest) {
            let minutes: f64 = c[1].parse().unwrap_or(0.0);
            let seconds: f64 = c[2].parse().unwrap_or(0.0);
            times.push(minutes * 60.0 + seconds);
            rest = &rest[c[0].len()..];
        }
        let text = rest.trim();
        lines.extend(times.into_iter().map(|time| Line { time, text: text.to_string() }));
    }
    lines.sort_by(|a, b| a.time.total_cmp(&b.time));
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_lrc_with_repeated_timestamps() {
        let lines = parse_lrc("[ar:Someone]\n[00:12.50]First\n[01:02.00][00:30.00] Chorus\n[02:00.00]");
        let times: Vec<f64> = lines.iter().map(|l| l.time).collect();
        assert_eq!(times, vec![12.5, 30.0, 62.0, 120.0]);
        assert_eq!(lines[1].text, "Chorus");
        assert_eq!(lines[3].text, "");
    }

    #[test]
    fn cleans_video_titles() {
        assert_eq!(search_terms("Song Name (Official Music Video)", "Artist - Topic"), ("Song Name".into(), "Artist".into()));
        assert_eq!(search_terms("ArtistVEVO - Hit [Lyrics] ft. Other", "ArtistVEVO"), ("Hit".into(), "Artist".into()));
        assert_eq!(search_terms("Track", "A, B & C"), ("Track".into(), "A".into()));
        assert_eq!(search_terms("Live - Forever (Remastered 2011)", "Other Band"), ("Live - Forever".into(), "Other Band".into()));
    }

    #[test]
    fn prefers_synced_match_of_same_length() {
        let rec = |d: f64, synced: bool| Record {
            duration: Some(d),
            instrumental: None,
            plain_lyrics: Some("x".into()),
            synced_lyrics: synced.then(|| "[00:01.00]x".into()),
        };
        let best = best_match(vec![rec(300.0, true), rec(200.0, false), rec(201.0, true)], 200.0).unwrap();
        assert_eq!(best.duration, Some(201.0));
    }
}
