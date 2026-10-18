//! The YouTube Music webview: creation, navigation policy and remote control.

pub mod bridge;

use crate::settings::Settings;
use crate::{app_ctx, windows};
use serde::Serialize;
use serde_json::{json, Value};
use tauri::webview::{NewWindowResponse, PageLoadEvent};
use tauri::{AppHandle, Manager, Runtime, Url, Webview, WebviewBuilder, WebviewUrl};

pub const YTM_HOME: &str = "https://music.youtube.com/";
pub const LABEL: &str = "ytm";
const MODERN_THEME: &str = include_str!("theme.css");
/// Third-party performance patch for YTM's page (from ytmdesktop2): rm3, which recycles
/// Polymer components instead of leaking them. Exposed as `window.__YTMR_PERF__`, which the
/// bridge calls once the player is ready. ytmdesktop2's CPU tamer is deliberately left out:
/// while music plays it holds the page's chained timers to one per animation frame, and YTM
/// renders long lists (History, playlists) through those timers, so pages filled slowly.
const PERF_SCRIPT: &str = concat!(
    "(() => {
",
    include_str!("vendor/rm3.js"),
    r#"
window.__YTMR_PERF__ = () => {
  try { injectRm3(); } catch (e) { console.warn("YTMusic Redux: rm3 skipped", e); }
};
})();
"#
);

/// The Modern UI stylesheet when enabled.
pub fn modern_css(app: &AppHandle) -> Option<&'static str> {
    app_ctx(app).settings.get().appearance.modern_ui.then_some(MODERN_THEME)
}

/// Video and audio preferences applied by the bridge (video toggle and audio engine).
pub fn audio_config(s: &Settings) -> Value {
    json!({
        "showVideos": s.playback.show_videos,
        "playbackRate": s.playback.playback_rate,
        "crossfadeSeconds": s.playback.crossfade_seconds,
        "skipDisliked": s.playback.skip_disliked,
        "videoQuality": s.playback.video_quality,
        "equalizer": s.playback.equalizer,
        "eqBands": crate::settings::EQ_BANDS,
    })
}
const LOGIN_URL: &str = "https://accounts.google.com/ServiceLogin?ltmpl=music&service=youtube&continue=https%3A%2F%2Fwww.youtube.com%2Fsignin%3Faction_handle_signin%3Dtrue%26app%3Ddesktop%26next%3Dhttps%253A%252F%252Fmusic.youtube.com%252F";

/// Every command YTM can be driven with (tray, shortcuts, UI, companion API, deep links).
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    PlayPause,
    Play,
    Pause,
    VolumeUp,
    VolumeDown,
    SetVolume(u8),
    Mute,
    Unmute,
    SeekTo(f64),
    Next,
    Previous,
    /// "NONE" | "ALL" | "ONE"
    RepeatMode(&'static str),
    Shuffle,
    PlayQueueIndex(usize),
    ChangeVideo { video_id: Option<String>, playlist_id: Option<String> },
    ToggleLike,
    ToggleDislike,
}

impl Command {
    /// Parses the argument-less commands used by shortcuts, tray and taskbar.
    pub fn simple(name: &str) -> Option<Self> {
        Some(match name {
            "playPause" => Self::PlayPause,
            "play" => Self::Play,
            "pause" => Self::Pause,
            "volumeUp" => Self::VolumeUp,
            "volumeDown" => Self::VolumeDown,
            "mute" => Self::Mute,
            "unmute" => Self::Unmute,
            "next" => Self::Next,
            "previous" => Self::Previous,
            "shuffle" => Self::Shuffle,
            "toggleLike" => Self::ToggleLike,
            "toggleDislike" => Self::ToggleDislike,
            _ => return None,
        })
    }

    fn to_js(&self) -> (&'static str, Value) {
        match self {
            Self::PlayPause => ("playPause", Value::Null),
            Self::Play => ("play", Value::Null),
            Self::Pause => ("pause", Value::Null),
            Self::VolumeUp => ("volumeUp", Value::Null),
            Self::VolumeDown => ("volumeDown", Value::Null),
            Self::SetVolume(v) => ("setVolume", json!(v)),
            Self::Mute => ("mute", Value::Null),
            Self::Unmute => ("unmute", Value::Null),
            Self::SeekTo(p) => ("seekTo", json!(p)),
            Self::Next => ("next", Value::Null),
            Self::Previous => ("previous", Value::Null),
            Self::RepeatMode(m) => ("repeatMode", json!(m)),
            Self::Shuffle => ("shuffle", Value::Null),
            Self::PlayQueueIndex(i) => ("playQueueIndex", json!(i)),
            Self::ChangeVideo { video_id, playlist_id } => {
                ("navigate", json!({ "watchEndpoint": { "videoId": video_id, "playlistId": playlist_id } }))
            }
            Self::ToggleLike => ("toggleLike", Value::Null),
            Self::ToggleDislike => ("toggleDislike", Value::Null),
        }
    }
}

pub fn webview<R: Runtime>(app: &AppHandle<R>) -> Option<Webview<R>> {
    app.get_webview(LABEL)
}

/// Calls a `window.__YTMR__` function with JSON-encoded arguments.
pub fn call<R: Runtime>(app: &AppHandle<R>, function: &str, args: &[Value]) {
    let Some(view) = webview(app) else { return };
    let args = args.iter().map(Value::to_string).collect::<Vec<_>>().join(",");
    let js = format!("window.__YTMR__ && window.__YTMR__.{function}({args});");
    if let Err(e) = view.eval(js) {
        log::warn!("YTM eval failed: {e}");
    }
}

pub fn execute<R: Runtime>(app: &AppHandle<R>, command: &Command) {
    let (name, value) = command.to_js();
    call(app, "exec", &[json!(name), value]);
}

const GOOGLE_ACCOUNT_TLDS: &str = "com ad ae com.af com.ag al am co.ao com.ar as at com.au az ba com.bd be bf bg com.bh bi bj com.bn com.bo com.br bs bt co.bw by com.bz ca cd cf cg ch ci co.ck cl cm cn com.co co.cr com.cu cv com.cy cz de dj dk dm com.do dz com.ec ee com.eg es com.et fi com.fj fm fr ga ge gg com.gh com.gi gl gm gr com.gt gy com.hk hn hr ht hu co.id ie co.il im co.in iq is it je com.jm jo co.jp co.ke com.kh ki kg co.kr com.kw kz la com.lb li lk co.ls lt lu lv com.ly co.ma md me mg mk ml com.mm mn com.mt mu mv mw com.mx com.my co.mz com.na com.ng com.ni ne nl no com.np nr nu co.nz com.om com.pa com.pe com.pg com.ph com.pk pl pn com.pr ps pt com.py com.qa ro ru rw com.sa com.sb sc se com.sg sh si sk com.sl sn so sm sr st com.sv td tg co.th com.tj tl tm tn to com.tr tt com.tw co.tz com.ua co.ug co.uk com.uy co.uz com.vc co.ve co.vi com.vn vu ws rs co.za co.zm co.zw cat";

fn is_google_accounts(host: &str) -> bool {
    host.strip_prefix("accounts.google.")
        .is_some_and(|tld| GOOGLE_ACCOUNT_TLDS.split(' ').any(|t| t == tld))
}

fn is_premium_page(url: &Url) -> bool {
    matches!(url.host_str(), Some("www.youtube.com" | "youtube.com")) && matches!(url.path(), "/premium" | "/musicpremium")
}

/// Port of upstream `isPreventedNavOrRedirect`.
pub fn is_allowed_navigation(url: &Url) -> bool {
    if url.scheme() != "https" {
        return false;
    }
    let host = url.host_str().unwrap_or_default();
    matches!(host, "consent.youtube.com" | "accounts.youtube.com" | "music.youtube.com")
        || (matches!(host, "www.youtube.com" | "youtube.com")
            && matches!(url.path(), "/signin" | "/premium" | "/musicpremium" | "/signin_prompt"))
        || is_google_accounts(host)
}

/// Opens links leaving YTM in the system browser, but only for Google/YouTube domains.
fn open_external(app: &AppHandle, url: &Url) {
    let host = url.host_str().unwrap_or_default();
    let mut parts = host.rsplit('.');
    let domain = match (parts.next(), parts.next()) {
        (Some(tld), Some(name)) => format!("{name}.{tld}"),
        _ => return,
    };
    if (domain == "google.com" || domain == "youtube.com") && url.scheme() == "https" {
        use tauri_plugin_opener::OpenerExt;
        let _ = app.opener().open_url(url.as_str(), None::<&str>);
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgeConfig {
    pub continue_where_you_left_off: bool,
    pub last_url: String,
    pub last_video_id: String,
    pub last_playlist_id: String,
    pub always_show_volume_slider: bool,
    pub ratio_volume: bool,
    pub custom_css: Option<String>,
    pub modern_css: Option<&'static str>,
    pub audio: Value,
}

/// Creates (or recreates) the YTM webview inside the main window.
pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let ctx = app_ctx(app);
    let settings = ctx.settings.get();
    let Some(window) = app.get_window(windows::MAIN) else { return Ok(()) };

    ctx.memory.update(app, |m| {
        m.ytm_view_loading = true;
        m.ytm_view_load_timedout = false;
        m.ytm_view_loading_status = "Initializing...".into();
        m.ytm_owns_header = false;
    });

    let start_url = ctx
        .state
        .read(|s| s.last_url.clone())
        .parse::<Url>()
        .ok()
        .filter(|u| settings.playback.continue_where_you_left_off && u.as_str().starts_with(YTM_HOME))
        .unwrap_or_else(|| YTM_HOME.parse().unwrap());

    let nav_app = app.clone();
    let window_app = app.clone();
    let title_app = app.clone();
    let load_app = app.clone();
    // The theme is applied at document start so YTM never paints unstyled; the bridge
    // config fetched later corrects it if the setting changed since this view was created.
    let theme = serde_json::to_string(&modern_css(app)).unwrap_or_else(|_| "null".into());
    let adblock = settings.playback.block_ads;
    let builder = WebviewBuilder::new(LABEL, WebviewUrl::External(start_url))
        .initialization_script(format!("window.__YTMR_THEME__ = {theme};
window.__YTMR_ADBLOCK__ = {adblock};
{PERF_SCRIPT}
{}", bridge::SCRIPT))
        .additional_browser_args(&ctx.browser_args)
        .background_color(windows::BACKGROUND)
        .devtools(settings.developer.enable_dev_tools)
        .on_navigation(move |url| {
            if is_premium_page(url) {
                // Premium pages need a signed-in session; route through the music login.
                let app = nav_app.clone();
                tauri::async_runtime::spawn(async move {
                    if let Some(view) = webview(&app) {
                        let _ = view.navigate(LOGIN_URL.parse().unwrap());
                    }
                });
                return false;
            }
            let allowed = is_allowed_navigation(url);
            if !allowed {
                log::info!("Blocking YTM navigation to {}", url.origin().ascii_serialization());
                open_external(&nav_app, url);
            }
            allowed
        })
        .on_new_window(move |url, _| {
            open_external(&window_app, &url);
            NewWindowResponse::Deny
        })
        .on_document_title_changed(move |_, title| {
            if let Some(w) = title_app.get_window(windows::MAIN) {
                let _ = w.set_title(&format!("{title} | YTMusic Redux"));
            }
        })
        .on_page_load(move |view, payload| {
            let app = &load_app;
            let ctx = app_ctx(app);
            match payload.event() {
                PageLoadEvent::Started => {
                    // A new document (e.g. a login page) has no bridge header until it reports in.
                    ctx.memory.update(app, |m| {
                        m.ytm_view_loading_status = "Loading YouTube Music...".into();
                        m.ytm_owns_header = false;
                    });
                    windows::layout_main(app);
                }
                PageLoadEvent::Finished => {
                    let host = payload.url().host_str().unwrap_or_default().to_string();
                    // Login and consent pages have no bridge; show them so the user can proceed.
                    if host == "consent.youtube.com" || is_google_accounts(&host) {
                        mark_loaded(app);
                    }
                    ctx.memory.update(app, |m| m.ytm_view_loading_status = "Loaded YouTube Music".into());
                    let _ = view.set_zoom(f64::from(ctx.settings.get().appearance.zoom) / 100.0);
                }
            }
        });

    let ytm = window.add_child(builder, tauri::LogicalPosition::new(0.0, 0.0), tauri::LogicalSize::new(1.0, 1.0))?;
    ytm.hide()?;
    let _ = ytm.set_zoom(f64::from(settings.appearance.zoom) / 100.0);

    let timeout_app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(30)).await;
        let ctx = app_ctx(&timeout_app);
        if ctx.memory.read(|m| m.ytm_view_loading) {
            ctx.memory.update(&timeout_app, |m| m.ytm_view_load_timedout = true);
        }
    });
    Ok(())
}

/// Reveals the YTM view once the bridge (or a login page) finished loading.
pub fn mark_loaded(app: &AppHandle) {
    let ctx = app_ctx(app);
    ctx.memory.update(app, |m| {
        m.ytm_view_loading = false;
        m.ytm_view_load_timedout = false;
    });
    windows::layout_main(app);
}

pub fn recreate(app: &AppHandle) -> tauri::Result<()> {
    if let Some(view) = webview(app) {
        view.close()?;
    }
    create(app)
}

/// Goes to YTM's home page inside the running app. Reloading the URL instead would restart
/// the whole page, which takes seconds; the full load remains the fallback while the bridge
/// is not up (a stuck first load, or a login page).
pub fn navigate_home(app: &AppHandle) {
    let Some(view) = webview(app) else { return };
    if app_ctx(app).memory.read(|m| m.ytm_view_loading) {
        let _ = view.navigate(YTM_HOME.parse().unwrap());
    } else {
        call(app, "navigateHome", &[]);
    }
}

/// What a shared link opens: a track, or any other YTM page.
#[derive(Debug, PartialEq)]
pub enum SharedLink {
    Track { video_id: String, playlist_id: Option<String> },
    Page(Url),
}

/// Maps a YouTube Music, YouTube or youtu.be URL (any scheme, so `ytmr://` swapped in for
/// `https://` works) to what it opens in YTM. Radio and mix ids (`RD…`) are dropped from
/// track links: YTM then shows "Radio is starting…" instead of playing the track.
pub fn shared_link(url: &Url) -> Option<SharedLink> {
    let host = url.host_str()?.trim_start_matches("www.").trim_start_matches("m.");
    let query = |key: &str| url.query_pairs().find(|(k, _)| k == key).map(|(_, v)| v.into_owned()).filter(|v| !v.is_empty());
    let segments: Vec<&str> = url.path_segments().map(|s| s.filter(|p| !p.is_empty()).collect()).unwrap_or_default();
    let playlist_id = query("list").filter(|l| !l.starts_with("RD"));
    let track = |video_id: &str| SharedLink::Track { video_id: video_id.to_string(), playlist_id: playlist_id.clone() };
    let page = |path: &str| format!("{YTM_HOME}{path}").parse().ok().map(SharedLink::Page);
    match (host, segments.as_slice()) {
        ("youtu.be", [id]) => Some(track(id)),
        ("youtube.com" | "music.youtube.com", ["watch"]) => query("v").map(|v| track(&v)),
        ("youtube.com" | "music.youtube.com", ["shorts" | "embed", id]) => Some(track(id)),
        ("youtube.com" | "music.youtube.com", ["playlist"]) => page(&format!("playlist?list={}", query("list")?)),
        ("youtube.com" | "music.youtube.com", ["channel", id]) => page(&format!("channel/{id}")),
        ("youtube.com" | "music.youtube.com", [handle]) if handle.starts_with('@') => page(handle),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_shared_links() {
        let track = |v: &str, l: Option<&str>| Some(SharedLink::Track { video_id: v.into(), playlist_id: l.map(Into::into) });
        let page = |p: &str| Some(SharedLink::Page(format!("{YTM_HOME}{p}").parse().unwrap()));
        assert_eq!(shared_link(&u("ytmr://music.youtube.com/watch?v=abc&list=PL1")), track("abc", Some("PL1")));
        assert_eq!(shared_link(&u("ytmr://www.youtube.com/watch?v=abc&list=RDAMVMabc")), track("abc", None));
        assert_eq!(shared_link(&u("ytmr://youtu.be/abc")), track("abc", None));
        assert_eq!(shared_link(&u("ytmr://m.youtube.com/shorts/abc")), track("abc", None));
        assert_eq!(shared_link(&u("ytmr://music.youtube.com/playlist?list=PL1")), page("playlist?list=PL1"));
        assert_eq!(shared_link(&u("ytmr://www.youtube.com/channel/UC1")), page("channel/UC1"));
        assert_eq!(shared_link(&u("ytmr://youtube.com/@artist")), page("@artist"));
        assert_eq!(shared_link(&u("ytmr://music.youtube.com/")), None);
        assert_eq!(shared_link(&u("ytmr://example.com/watch?v=abc")), None);
    }

    fn u(s: &str) -> Url {
        s.parse().unwrap()
    }

    #[test]
    fn navigation_policy_matches_upstream() {
        assert!(is_allowed_navigation(&u("https://music.youtube.com/watch?v=x")));
        assert!(is_allowed_navigation(&u("https://accounts.google.co.uk/signin")));
        assert!(is_allowed_navigation(&u("https://www.youtube.com/signin")));
        assert!(!is_allowed_navigation(&u("https://www.youtube.com/watch?v=x")));
        assert!(!is_allowed_navigation(&u("https://accounts.google.evil/")));
        assert!(!is_allowed_navigation(&u("https://example.com/")));
        assert!(!is_allowed_navigation(&u("http://music.youtube.com/")));
        assert!(is_premium_page(&u("https://youtube.com/musicpremium")));
    }

    #[test]
    fn commands_map_to_bridge_calls() {
        let (name, v) = Command::ChangeVideo { video_id: Some("a".into()), playlist_id: None }.to_js();
        assert_eq!(name, "navigate");
        assert_eq!(v["watchEndpoint"]["videoId"], "a");
        assert_eq!(Command::simple("toggleLike"), Some(Command::ToggleLike));
        assert_eq!(Command::simple("seekTo"), None);
    }
}
