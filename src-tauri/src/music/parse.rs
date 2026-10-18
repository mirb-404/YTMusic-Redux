//! Turns YouTube Music's web client responses (`/youtubei/v1/*`, the renderer trees
//! music.youtube.com itself draws) into the small models the app's own UI renders.
//!
//! Endpoints stay YTM's own JSON, trimmed to what is needed to follow them: the UI hands
//! a browse endpoint back to `music_browse`, and a watch endpoint to the YTM page, which
//! plays it exactly as a click in YTM would.

use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Page {
    pub header: Option<Header>,
    /// Filter or mood chips above the sections (search filters, home moods).
    pub chips: Vec<Chip>,
    pub sections: Vec<Section>,
    pub continuation: Option<Continuation>,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Header {
    pub kind: HeaderKind,
    pub title: String,
    pub subtitle: Vec<Run>,
    pub second_subtitle: Vec<Run>,
    pub description: Option<String>,
    pub thumbnails: Vec<Thumbnail>,
    pub play: Option<Value>,
    pub shuffle: Option<Value>,
    pub radio: Option<Value>,
    /// Save to (or remove from) the library.
    pub save: Option<MenuEntry>,
    pub menu: Vec<MenuEntry>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum HeaderKind {
    /// A page title only (library, history, moods).
    #[default]
    Plain,
    /// Album or playlist: artwork beside title and actions.
    Collection,
    /// Artist: wide banner behind the name.
    Artist,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Chip {
    pub text: String,
    pub endpoint: Option<Value>,
    pub selected: bool,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Section {
    pub kind: SectionKind,
    pub title: String,
    pub strapline: Option<String>,
    /// "More" / "See all".
    pub more: Option<Value>,
    pub items: Vec<Item>,
    /// Description shelves (artist "About").
    pub text: Option<String>,
    /// A carousel of track rows (Trending, Quick picks) rather than cards.
    pub rows: bool,
    pub continuation: Option<Continuation>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SectionKind {
    /// A horizontal row of cards.
    #[default]
    Carousel,
    /// Track rows.
    List,
    /// Wrapping cards.
    Grid,
    /// Search's top result, drawn large.
    Card,
    Description,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    pub kind: ItemKind,
    pub title: String,
    pub subtitle: Vec<Run>,
    pub thumbnails: Vec<Thumbnail>,
    /// Where clicking the item goes: a browse endpoint (album, artist, playlist) or a
    /// watch endpoint (song, video).
    pub endpoint: Option<Value>,
    /// Plays the item from its card's play button (albums and playlists).
    pub play: Option<Value>,
    pub video_id: Option<String>,
    pub duration: Option<String>,
    pub index: Option<String>,
    pub aspect: Aspect,
    /// Mood and genre buttons carry a color stripe.
    pub color: Option<String>,
    pub menu: Vec<MenuEntry>,
    pub like: Option<Like>,
}

/// One entry of an item's "more" menu, as YTM offers it for that item.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MenuEntry {
    pub text: String,
    pub icon: String,
    pub action: MenuAction,
    /// On/off entries (save to library, pin): the other state's text and action.
    pub toggle: Option<Toggle>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Toggle {
    pub toggled: bool,
    pub text: String,
    pub icon: String,
    pub action: MenuAction,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum MenuAction {
    /// Opens a page or plays (radio, shuffle, go to artist or album).
    Open { endpoint: Value },
    /// Play next, or add to the end of the queue.
    Queue { next: bool, video_id: Option<String>, playlist_id: Option<String> },
    /// A request YTM sends as is: like, library feedback, playlist edits.
    Service { endpoint: Value },
    AddToPlaylist { video_id: Option<String>, playlist_id: Option<String> },
    Share,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Like {
    pub video_id: String,
    /// "LIKE", "DISLIKE" or "INDIFFERENT".
    pub status: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ItemKind {
    Song,
    Video,
    Album,
    Playlist,
    Artist,
    Episode,
    Podcast,
    /// Mood and genre buttons.
    Category,
    #[default]
    Other,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Aspect {
    #[default]
    Square,
    Wide,
    Circle,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Run {
    pub text: String,
    pub endpoint: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Thumbnail {
    pub url: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Continuation {
    pub token: String,
    /// Section lists continue through a `ctoken` query; shelves through a request body.
    pub query: bool,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Suggestions {
    pub queries: Vec<Vec<Run>>,
    pub items: Vec<Item>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GuideEntry {
    pub title: String,
    pub subtitle: String,
    pub endpoint: Value,
    pub play: Option<Value>,
}

// --- helpers

fn get<'a>(v: &'a Value, path: &[&str]) -> Option<&'a Value> {
    path.iter().try_fold(v, |v, k| match k.parse::<usize>() {
        Ok(i) if v.is_array() => v.get(i),
        _ => v.get(*k),
    })
}

fn str_at(v: &Value, path: &[&str]) -> Option<String> {
    get(v, path).and_then(Value::as_str).map(str::to_string)
}

fn arr<'a>(v: Option<&'a Value>) -> &'a [Value] {
    v.and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[])
}

/// A renderer wrapper's single key and value: `{"musicShelfRenderer": {...}}`.
fn renderer(v: &Value) -> Option<(&str, &Value)> {
    v.as_object()?.iter().next().map(|(k, v)| (k.as_str(), v))
}

/// Keeps only the parts of an endpoint needed to follow it.
pub fn endpoint(v: Option<&Value>) -> Option<Value> {
    let v = v?.as_object()?;
    for key in ["browseEndpoint", "watchEndpoint", "watchPlaylistEndpoint", "searchEndpoint"] {
        let Some(inner) = v.get(key).and_then(Value::as_object) else { continue };
        let mut kept = Map::new();
        for field in ["browseId", "params", "videoId", "playlistId", "index", "query"] {
            if let Some(x) = inner.get(field) {
                kept.insert(field.into(), x.clone());
            }
        }
        if let Some(page_type) = get(&Value::Object(inner.clone()), &["browseEndpointContextSupportedConfigs", "browseEndpointContextMusicConfig", "pageType"]) {
            kept.insert("pageType".into(), page_type.clone());
        }
        if let Some(video_type) = get(&Value::Object(inner.clone()), &["watchEndpointMusicSupportedConfigs", "watchEndpointMusicConfig", "musicVideoType"]) {
            kept.insert("musicVideoType".into(), video_type.clone());
        }
        return Some(json!({ key: kept }));
    }
    None
}

pub fn runs(v: Option<&Value>) -> Vec<Run> {
    let Some(v) = v else { return Vec::new() };
    if let Some(s) = v.get("simpleText").and_then(Value::as_str) {
        return vec![Run { text: s.into(), endpoint: None }];
    }
    arr(v.get("runs"))
        .iter()
        .map(|r| Run { text: str_at(r, &["text"]).unwrap_or_default(), endpoint: endpoint(r.get("navigationEndpoint")) })
        .collect()
}

fn text(v: Option<&Value>) -> String {
    runs(v).into_iter().map(|r| r.text).collect()
}

fn thumbnails(v: &Value) -> Vec<Thumbnail> {
    for path in [
        &["thumbnailRenderer", "musicThumbnailRenderer", "thumbnail", "thumbnails"][..],
        &["thumbnail", "musicThumbnailRenderer", "thumbnail", "thumbnails"],
        &["thumbnail", "croppedSquareThumbnailRenderer", "thumbnail", "thumbnails"],
        &["thumbnailRenderer", "croppedSquareThumbnailRenderer", "thumbnail", "thumbnails"],
        &["thumbnail", "thumbnails"],
        &["foregroundThumbnail", "musicThumbnailRenderer", "thumbnail", "thumbnails"],
    ] {
        let list = arr(get(v, path));
        if !list.is_empty() {
            return list
                .iter()
                .filter_map(|t| {
                    Some(Thumbnail {
                        url: str_at(t, &["url"])?,
                        width: t.get("width").and_then(Value::as_u64).unwrap_or(0) as u32,
                        height: t.get("height").and_then(Value::as_u64).unwrap_or(0) as u32,
                    })
                })
                .collect();
        }
    }
    Vec::new()
}

fn page_type(endpoint: &Option<Value>) -> Option<&str> {
    endpoint.as_ref()?.get("browseEndpoint")?.get("pageType")?.as_str()
}

fn kind_of(endpoint: &Option<Value>) -> ItemKind {
    if let Some(e) = endpoint {
        if let Some(t) = e.get("watchEndpoint").and_then(|w| w.get("musicVideoType")).and_then(Value::as_str) {
            return match t {
                "MUSIC_VIDEO_TYPE_ATV" => ItemKind::Song,
                "MUSIC_VIDEO_TYPE_PODCAST_EPISODE" => ItemKind::Episode,
                _ => ItemKind::Video,
            };
        }
        if e.get("watchEndpoint").is_some() {
            return ItemKind::Song;
        }
    }
    match page_type(endpoint) {
        Some("MUSIC_PAGE_TYPE_ALBUM" | "MUSIC_PAGE_TYPE_AUDIOBOOK") => ItemKind::Album,
        Some("MUSIC_PAGE_TYPE_PLAYLIST") => ItemKind::Playlist,
        Some("MUSIC_PAGE_TYPE_ARTIST" | "MUSIC_PAGE_TYPE_LIBRARY_ARTIST" | "MUSIC_PAGE_TYPE_USER_CHANNEL") => ItemKind::Artist,
        Some("MUSIC_PAGE_TYPE_PODCAST_SHOW_DETAIL_PAGE") => ItemKind::Podcast,
        Some("MUSIC_PAGE_TYPE_NON_MUSIC_AUDIO_TRACK_PAGE") => ItemKind::Episode,
        _ => ItemKind::Other,
    }
}

fn play_button(v: &Value) -> Option<Value> {
    endpoint(get(v, &["thumbnailOverlay", "musicItemThumbnailOverlayRenderer", "content", "musicPlayButtonRenderer", "playNavigationEndpoint"]))
        .or_else(|| endpoint(get(v, &["overlay", "musicItemThumbnailOverlayRenderer", "content", "musicPlayButtonRenderer", "playNavigationEndpoint"])))
}

fn aspect(kind: ItemKind, ratio: Option<&str>) -> Aspect {
    match (kind, ratio) {
        (ItemKind::Artist, _) => Aspect::Circle,
        (_, Some("MUSIC_TWO_ROW_ITEM_THUMBNAIL_ASPECT_RATIO_RECTANGLE_16_9")) | (ItemKind::Video | ItemKind::Episode, None) => Aspect::Wide,
        _ => Aspect::Square,
    }
}

// --- menus

fn keep(v: &Value, fields: &[&str]) -> Value {
    let mut out = Map::new();
    for f in fields {
        if let Some(x) = v.get(*f) {
            out.insert((*f).into(), x.clone());
        }
    }
    Value::Object(out)
}

/// Requests a menu entry sends unchanged, trimmed to what they need.
pub fn service(v: Option<&Value>) -> Option<Value> {
    let v = v?;
    if let Some(l) = v.get("likeEndpoint") {
        l.get("status")?;
        return Some(json!({ "likeEndpoint": keep(l, &["status", "target"]) }));
    }
    if let Some(f) = v.get("feedbackEndpoint") {
        return Some(json!({ "feedbackEndpoint": { "feedbackToken": f.get("feedbackToken")? } }));
    }
    if let Some(p) = v.get("playlistEditEndpoint") {
        let actions: Vec<Value> =
            arr(p.get("actions")).iter().map(|a| keep(a, &["action", "setVideoId", "removedVideoId", "addedVideoId", "movedSetVideoIdSuccessor"])).collect();
        return Some(json!({ "playlistEditEndpoint": { "playlistId": p.get("playlistId")?, "actions": actions } }));
    }
    None
}

fn menu_action(v: Option<&Value>) -> Option<MenuAction> {
    let v = v?;
    let ids = |t: &Value| (str_at(t, &["videoId"]), str_at(t, &["playlistId"]));
    if let Some(q) = v.get("queueAddEndpoint") {
        let (video_id, playlist_id) = ids(q.get("queueTarget")?);
        let next = str_at(q, &["queueInsertPosition"]).as_deref() == Some("INSERT_AFTER_CURRENT_VIDEO");
        return Some(MenuAction::Queue { next, video_id, playlist_id });
    }
    if let Some(a) = v.get("addToPlaylistEndpoint") {
        let (video_id, playlist_id) = ids(a);
        return Some(MenuAction::AddToPlaylist { video_id, playlist_id });
    }
    if let Some(endpoint) = service(Some(v)) {
        return Some(MenuAction::Service { endpoint });
    }
    endpoint(Some(v)).map(|endpoint| MenuAction::Open { endpoint })
}

fn icon(v: Option<&Value>) -> String {
    v.and_then(|i| i.get("iconType")).and_then(Value::as_str).unwrap_or_default().to_string()
}

/// An on/off entry: the entry is what selecting it does now, the toggle the state after.
fn toggle_entry(toggled: bool, off: (String, String, MenuAction), on: (String, String, MenuAction)) -> MenuEntry {
    let (now, after) = if toggled { (on, off) } else { (off, on) };
    MenuEntry { text: now.0, icon: now.1, action: now.2, toggle: Some(Toggle { toggled, text: after.0, icon: after.1, action: after.2 }) }
}

fn menu_entry(v: &Value) -> Option<MenuEntry> {
    let (key, e) = renderer(v)?;
    match key {
        "menuNavigationItemRenderer" | "menuServiceItemRenderer" => {
            let icon = icon(e.get("icon"));
            let action = menu_action(e.get("navigationEndpoint").or_else(|| e.get("serviceEndpoint"))).or_else(|| (icon == "SHARE").then_some(MenuAction::Share))?;
            Some(MenuEntry { text: text(e.get("text")), icon, action, toggle: None })
        }
        "toggleMenuServiceItemRenderer" => {
            let toggled = e.get("isToggled").and_then(Value::as_bool).unwrap_or(false);
            let off = (text(e.get("defaultText")), icon(e.get("defaultIcon")), menu_action(e.get("defaultServiceEndpoint"))?);
            let on = (text(e.get("toggledText")), icon(e.get("toggledIcon")), menu_action(e.get("toggledServiceEndpoint"))?);
            Some(toggle_entry(toggled, off, on))
        }
        _ => None,
    }
}

fn menu(v: &Value) -> (Vec<MenuEntry>, Option<Like>) {
    let Some(m) = get(v, &["menu", "menuRenderer"]) else { return (Vec::new(), None) };
    let entries = arr(m.get("items")).iter().filter_map(menu_entry).collect();
    let like = arr(m.get("topLevelButtons")).iter().find_map(|b| b.get("likeButtonRenderer")).and_then(|l| {
        Some(Like { video_id: str_at(l, &["target", "videoId"])?, status: str_at(l, &["likeStatus"]).unwrap_or_else(|| "INDIFFERENT".into()) })
    });
    (entries, like)
}

// --- items

fn two_row_item(v: &Value) -> Item {
    let endpoint = endpoint(v.get("navigationEndpoint"));
    let kind = kind_of(&endpoint);
    let (menu, like) = menu(v);
    Item {
        menu,
        like,
        kind,
        title: text(v.get("title")),
        subtitle: runs(v.get("subtitle")),
        thumbnails: thumbnails(v),
        play: play_button(v),
        video_id: endpoint.as_ref().and_then(|e| str_at(e, &["watchEndpoint", "videoId"])),
        aspect: aspect(kind, v.get("aspectRatio").and_then(Value::as_str)),
        endpoint,
        ..Default::default()
    }
}

fn list_item(v: &Value) -> Item {
    let columns = arr(v.get("flexColumns"));
    let column = |i: usize| columns.get(i).and_then(|c| get(c, &["musicResponsiveListItemFlexColumnRenderer", "text"]));
    let title_runs = runs(column(0));
    let mut subtitle = Vec::new();
    for i in 1..columns.len() {
        let r = runs(column(i));
        if r.iter().all(|r| r.text.trim().is_empty()) {
            continue;
        }
        if !subtitle.is_empty() {
            subtitle.push(Run { text: " • ".into(), endpoint: None });
        }
        subtitle.extend(r);
    }
    let video_id = str_at(v, &["playlistItemData", "videoId"]);
    let endpoint = endpoint(v.get("navigationEndpoint")).or_else(|| title_runs.first().and_then(|r| r.endpoint.clone())).or_else(|| {
        video_id.as_ref().map(|id| json!({ "watchEndpoint": { "videoId": id } }))
    });
    let kind = kind_of(&endpoint);
    let duration = arr(v.get("fixedColumns"))
        .first()
        .map(|c| text(get(c, &["musicResponsiveListItemFixedColumnRenderer", "text"])))
        .filter(|t| !t.is_empty());
    let index = Some(text(get(v, &["index"]))).filter(|t| !t.is_empty()).or_else(|| {
        Some(text(get(v, &["customIndexColumn", "musicCustomIndexColumnRenderer", "text"]))).filter(|t| !t.is_empty())
    });
    let (menu, like) = menu(v);
    Item {
        menu,
        like,
        kind,
        title: title_runs.into_iter().map(|r| r.text).collect(),
        subtitle,
        thumbnails: thumbnails(v),
        play: play_button(v),
        video_id: video_id.or_else(|| endpoint.as_ref().and_then(|e| str_at(e, &["watchEndpoint", "videoId"]))),
        duration,
        index,
        aspect: aspect(kind, None),
        endpoint,
        ..Default::default()
    }
}

fn navigation_button(v: &Value) -> Item {
    Item {
        kind: ItemKind::Category,
        title: text(v.get("buttonText")),
        endpoint: endpoint(v.get("clickCommand")),
        color: v.get("solid").and_then(|s| s.get("leftStripeColor")).and_then(Value::as_u64).map(argb_to_css),
        ..Default::default()
    }
}

/// YTM colors are ARGB integers.
fn argb_to_css(argb: u64) -> String {
    format!("#{:06x}", argb & 0xff_ffff)
}

fn item(v: &Value) -> Option<Item> {
    let (key, inner) = renderer(v)?;
    Some(match key {
        "musicTwoRowItemRenderer" => two_row_item(inner),
        "musicResponsiveListItemRenderer" => list_item(inner),
        "musicNavigationButtonRenderer" => navigation_button(inner),
        "musicMultiRowListItemRenderer" => {
            let endpoint = endpoint(inner.get("onTap")).or_else(|| endpoint(get(inner, &["title", "runs", "0", "navigationEndpoint"])));
            Item {
                kind: ItemKind::Episode,
                title: text(inner.get("title")),
                subtitle: runs(inner.get("subtitle")),
                thumbnails: thumbnails(inner),
                video_id: endpoint.as_ref().and_then(|e| str_at(e, &["watchEndpoint", "videoId"])),
                aspect: Aspect::Square,
                endpoint,
                ..Default::default()
            }
        }
        _ => return None,
    })
}

fn items(list: &[Value]) -> Vec<Item> {
    list.iter().filter_map(item).filter(|i| !i.title.is_empty()).collect()
}

/// A shelf's continuation: a trailing `continuationItemRenderer`, or `continuations`.
fn continuation(v: &Value) -> Option<Continuation> {
    if let Some(token) = arr(v.get("contents"))
        .iter()
        .chain(arr(v.get("items")))
        .find_map(|c| str_at(c, &["continuationItemRenderer", "continuationEndpoint", "continuationCommand", "token"]))
    {
        return Some(Continuation { token, query: false });
    }
    arr(v.get("continuations")).first().and_then(|c| {
        str_at(c, &["nextContinuationData", "continuation"]).map(|token| Continuation { token, query: true })
    })
}

// --- sections

fn section(v: &Value) -> Option<Section> {
    let (key, s) = renderer(v)?;
    Some(match key {
        "musicCarouselShelfRenderer" => {
            let header = get(s, &["header", "musicCarouselShelfBasicHeaderRenderer"]);
            Section {
                kind: SectionKind::Carousel,
                title: header.map(|h| text(h.get("title"))).unwrap_or_default(),
                strapline: header.map(|h| text(h.get("strapline"))).filter(|t| !t.is_empty()),
                more: header.and_then(|h| {
                    endpoint(get(h, &["moreContentButton", "buttonRenderer", "navigationEndpoint"])).or_else(|| endpoint(get(h, &["title", "runs", "0", "navigationEndpoint"])))
                }),
                rows: arr(s.get("contents")).first().and_then(renderer).is_some_and(|(k, _)| k == "musicResponsiveListItemRenderer"),
                items: items(arr(s.get("contents"))),
                ..Default::default()
            }
        }
        "musicShelfRenderer" | "musicPlaylistShelfRenderer" => Section {
            kind: SectionKind::List,
            title: text(s.get("title")),
            more: endpoint(get(s, &["bottomEndpoint"])).or_else(|| endpoint(get(s, &["title", "runs", "0", "navigationEndpoint"]))),
            items: items(arr(s.get("contents"))),
            continuation: continuation(s),
            ..Default::default()
        },
        "gridRenderer" => Section {
            kind: SectionKind::Grid,
            title: text(get(s, &["header", "gridHeaderRenderer", "title"])),
            items: items(arr(s.get("items"))),
            continuation: continuation(s),
            ..Default::default()
        },
        "musicCardShelfRenderer" => {
            let endpoint = endpoint(get(s, &["title", "runs", "0", "navigationEndpoint"])).or_else(|| endpoint(s.get("onTap")));
            let kind = kind_of(&endpoint);
            let top = Item {
                kind,
                title: text(s.get("title")),
                subtitle: runs(s.get("subtitle")),
                thumbnails: thumbnails(s),
                video_id: endpoint.as_ref().and_then(|e| str_at(e, &["watchEndpoint", "videoId"])),
                play: arr(s.get("buttons")).iter().find_map(|b| self::endpoint(get(b, &["buttonRenderer", "command"]))),
                aspect: aspect(kind, None),
                endpoint,
                menu: menu(s).0,
                ..Default::default()
            };
            let mut list = vec![top];
            list.extend(items(arr(s.get("contents"))));
            Section { kind: SectionKind::Card, title: text(get(s, &["header", "musicCardShelfHeaderBasicRenderer", "title"])), items: list, ..Default::default() }
        }
        "musicDescriptionShelfRenderer" => Section {
            kind: SectionKind::Description,
            title: text(s.get("header")),
            text: Some(text(s.get("description"))).filter(|t| !t.is_empty()),
            ..Default::default()
        },
        // Search lists each result in its own item section.
        "itemSectionRenderer" => {
            let list = items(arr(s.get("contents")));
            if list.is_empty() {
                return None;
            }
            Section { kind: SectionKind::List, items: list, ..Default::default() }
        }
        _ => return None,
    })
}

/// Section list contents; consecutive untitled lists (search results) merge into one.
fn sections(list: &[Value]) -> Vec<Section> {
    let mut out: Vec<Section> = Vec::new();
    for s in list.iter().filter_map(section) {
        if let Some(last) = out.last_mut() {
            if s.kind == SectionKind::List && last.kind == SectionKind::List && s.title.is_empty() && last.title.is_empty() && last.continuation.is_none() {
                last.items.extend(s.items);
                continue;
            }
        }
        if s.kind == SectionKind::Description || !s.items.is_empty() {
            out.push(s);
        }
    }
    out
}

fn chips(section_list: &Value) -> Vec<Chip> {
    arr(get(section_list, &["header", "chipCloudRenderer", "chips"]))
        .iter()
        .filter_map(|c| c.get("chipCloudChipRenderer"))
        .map(|c| Chip {
            text: text(c.get("text")),
            endpoint: endpoint(c.get("navigationEndpoint")),
            selected: c.get("isSelected").and_then(Value::as_bool).unwrap_or(false),
        })
        .filter(|c| !c.text.is_empty())
        .collect()
}

// --- headers

fn responsive_header(h: &Value) -> Header {
    let buttons = arr(h.get("buttons"));
    let menu = buttons.iter().find_map(|b| get(b, &["menuRenderer", "items"])).map(|i| arr(Some(i))).unwrap_or(&[]);
    let menu_endpoint = |icon: &str| {
        menu.iter()
            .filter_map(|i| i.get("menuNavigationItemRenderer"))
            .find(|i| str_at(i, &["icon", "iconType"]).as_deref() == Some(icon))
            .and_then(|i| endpoint(i.get("navigationEndpoint")))
    };
    Header {
        kind: HeaderKind::Collection,
        title: text(h.get("title")),
        subtitle: runs(h.get("subtitle")),
        second_subtitle: {
            let mut r = runs(h.get("straplineTextOne"));
            let second = runs(h.get("secondSubtitle"));
            if !r.is_empty() && !second.is_empty() {
                r.push(Run { text: " • ".into(), endpoint: None });
            }
            r.extend(second);
            r
        },
        description: Some(text(get(h, &["description", "musicDescriptionShelfRenderer", "description"]))).filter(|t| !t.is_empty()),
        thumbnails: thumbnails(h),
        play: buttons.iter().find_map(|b| endpoint(get(b, &["musicPlayButtonRenderer", "playNavigationEndpoint"]))),
        shuffle: menu_endpoint("MUSIC_SHUFFLE"),
        radio: menu_endpoint("MIX"),
        save: buttons.iter().find_map(|b| b.get("toggleButtonRenderer")).and_then(|t| {
            let toggled = t.get("isToggled").and_then(Value::as_bool).unwrap_or(false);
            let add = ("Save to library".to_string(), "BOOKMARK_BORDER".to_string(), menu_action(t.get("defaultServiceEndpoint"))?);
            let remove = ("Remove from library".to_string(), "BOOKMARK".to_string(), menu_action(t.get("toggledServiceEndpoint"))?);
            Some(toggle_entry(toggled, add, remove))
        }),
        menu: menu.iter().filter_map(menu_entry).collect(),
    }
}

fn immersive_header(h: &Value) -> Header {
    Header {
        kind: HeaderKind::Artist,
        title: text(h.get("title")),
        subtitle: runs(get(h, &["subscriptionButton", "subscribeButtonRenderer", "longSubscriberCountText"])),
        description: Some(text(h.get("description"))).filter(|t| !t.is_empty()),
        thumbnails: thumbnails(h),
        shuffle: endpoint(get(h, &["playButton", "buttonRenderer", "navigationEndpoint"])),
        radio: endpoint(get(h, &["startRadioButton", "buttonRenderer", "navigationEndpoint"])),
        ..Default::default()
    }
}

fn header(v: &Value) -> Option<Header> {
    let (key, h) = renderer(v)?;
    match key {
        "musicResponsiveHeaderRenderer" | "musicEditablePlaylistDetailHeaderRenderer" => {
            let inner = get(h, &["header", "musicResponsiveHeaderRenderer"]).unwrap_or(h);
            Some(responsive_header(inner))
        }
        "musicImmersiveHeaderRenderer" | "musicVisualHeaderRenderer" => Some(immersive_header(h)),
        "musicHeaderRenderer" | "musicDetailHeaderRenderer" => Some(Header { title: text(h.get("title")), ..Default::default() }),
        _ => None,
    }
}

// --- responses

/// `browse` and `search` responses.
pub fn page(response: &Value) -> Page {
    let mut page = Page::default();
    if let Some(h) = response.get("header") {
        page.header = header(h);
    }
    let contents = response.get("contents");
    // Album and playlist pages: header on the left tab, tracks in the secondary contents.
    if let Some(two) = contents.and_then(|c| c.get("twoColumnBrowseResultsRenderer")) {
        let left = get(two, &["tabs", "0", "tabRenderer", "content", "sectionListRenderer"]);
        if let Some(left) = left {
            for c in arr(left.get("contents")) {
                if page.header.is_none() {
                    page.header = header(c);
                }
            }
        }
        if let Some(right) = get(two, &["secondaryContents", "sectionListRenderer"]) {
            page.sections = sections(arr(right.get("contents")));
            page.continuation = continuation(right);
        }
        return page;
    }
    let tab = contents.and_then(|c| {
        get(c, &["singleColumnBrowseResultsRenderer", "tabs", "0", "tabRenderer", "content"])
            .or_else(|| get(c, &["tabbedSearchResultsRenderer", "tabs", "0", "tabRenderer", "content"]))
    });
    if let Some(list) = tab.and_then(|t| t.get("sectionListRenderer")) {
        page.chips = chips(list);
        page.sections = sections(arr(list.get("contents")));
        page.continuation = continuation(list);
        if page.header.is_none() {
            page.header = list.get("header").and_then(header);
        }
    }
    page
}

/// Continuation responses: more sections for a page, or more items for a shelf (returned
/// as a single section).
pub fn continuation_page(response: &Value) -> Page {
    let mut page = Page::default();
    if let Some(c) = get(response, &["continuationContents", "sectionListContinuation"]) {
        page.sections = sections(arr(c.get("contents")));
        page.continuation = continuation(c);
        return page;
    }
    for key in ["musicShelfContinuation", "musicPlaylistShelfContinuation", "gridContinuation"] {
        if let Some(c) = get(response, &["continuationContents", key]) {
            let list = if key == "gridContinuation" { c.get("items") } else { c.get("contents") };
            page.sections = vec![Section { kind: SectionKind::List, items: items(arr(list)), continuation: continuation(c), ..Default::default() }];
            return page;
        }
    }
    let appended = arr(response.get("onResponseReceivedActions"))
        .iter()
        .find_map(|a| get(a, &["appendContinuationItemsAction", "continuationItems"]))
        .map(|v| arr(Some(v)))
        .unwrap_or(&[]);
    if !appended.is_empty() {
        let holder = json!({ "contents": appended });
        page.sections = vec![Section { kind: SectionKind::List, items: items(appended), continuation: continuation(&holder), ..Default::default() }];
    }
    page
}

pub fn suggestions(response: &Value) -> Suggestions {
    let mut out = Suggestions::default();
    for section in arr(response.get("contents")) {
        for c in arr(get(section, &["searchSuggestionsSectionRenderer", "contents"])) {
            if let Some(s) = c.get("searchSuggestionRenderer") {
                out.queries.push(runs(s.get("suggestion")));
            } else if let Some(i) = item(c) {
                out.items.push(i);
            }
        }
    }
    out
}

/// The sidebar: the user's playlists from the guide.
pub fn guide(response: &Value) -> Vec<GuideEntry> {
    arr(response.get("items"))
        .iter()
        .flat_map(|s| arr(get(s, &["guideSectionRenderer", "items"])))
        .filter_map(|e| e.get("guideEntryRenderer"))
        .filter_map(|e| {
            let endpoint = endpoint(e.get("navigationEndpoint"))?;
            (page_type(&Some(endpoint.clone())) == Some("MUSIC_PAGE_TYPE_PLAYLIST")).then(|| GuideEntry {
                title: text(e.get("formattedTitle")),
                subtitle: text(e.get("formattedSubtitle")),
                play: self::endpoint(get(e, &["mouseOverButton", "musicPlayButtonRenderer", "playNavigationEndpoint"])),
                endpoint,
            })
        })
        .collect()
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistTarget {
    pub playlist_id: String,
    pub title: String,
    /// The track (or every track of the list) is already in this playlist.
    pub contains: bool,
}

/// The user's playlists a track or list can be saved to.
pub fn playlist_targets(response: &Value) -> Vec<PlaylistTarget> {
    arr(response.get("contents"))
        .iter()
        .filter_map(|c| c.get("addToPlaylistRenderer"))
        .flat_map(|r| arr(r.get("playlists")))
        .filter_map(|p| p.get("playlistAddToOptionRenderer"))
        .filter_map(|p| {
            Some(PlaylistTarget {
                playlist_id: str_at(p, &["playlistId"])?,
                title: text(p.get("title")),
                contains: str_at(p, &["containsSelectedVideos"]).as_deref() == Some("ALL"),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> Value {
        let path = format!("{}/tests/fixtures/music/{name}.json", env!("CARGO_MANIFEST_DIR"));
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
    }

    #[test]
    fn album_page() {
        let p = page(&fixture("album"));
        let h = p.header.expect("album header");
        assert_eq!(h.kind, HeaderKind::Collection);
        assert!(!h.title.is_empty() && !h.thumbnails.is_empty());
        assert!(h.play.as_ref().unwrap().get("watchPlaylistEndpoint").is_some());
        assert!(h.shuffle.is_some());
        let tracks = &p.sections[0];
        assert_eq!(tracks.kind, SectionKind::List);
        let first = &tracks.items[0];
        assert_eq!(first.index.as_deref(), Some("1"));
        assert!(first.duration.is_some() && first.video_id.is_some());
        assert!(first.endpoint.as_ref().unwrap().get("watchEndpoint").is_some());
    }

    #[test]
    fn artist_page() {
        let p = page(&fixture("artist"));
        let h = p.header.expect("artist header");
        assert_eq!(h.kind, HeaderKind::Artist);
        assert_eq!(h.title, "KANA-BOON");
        assert!(!h.thumbnails.is_empty());
        assert_eq!(p.sections[0].title, "Top songs");
        assert!(p.sections.iter().any(|s| s.kind == SectionKind::Carousel && s.items.iter().any(|i| i.kind == ItemKind::Album)));
        assert!(p.sections.iter().any(|s| s.kind == SectionKind::Description));
    }

    #[test]
    fn playlist_page_continues() {
        let p = page(&fixture("playlist"));
        assert_eq!(p.header.unwrap().kind, HeaderKind::Collection);
        let list = &p.sections[0];
        assert_eq!(list.items.len(), 6);
        let c = list.continuation.as_ref().expect("shelf continuation");
        assert!(!c.query);
    }

    #[test]
    fn explore_page() {
        let p = page(&fixture("explore"));
        let moods = p.sections.iter().find(|s| s.title == "Moods & genres").unwrap();
        assert!(moods.items.iter().all(|i| i.kind == ItemKind::Category && i.endpoint.is_some()));
        let trending = p.sections.iter().find(|s| s.title == "Trending").unwrap();
        assert!(trending.rows);
        assert_eq!(trending.items[0].index.as_deref(), Some("1"));
    }

    #[test]
    fn search_results() {
        let p = page(&fixture("search"));
        assert!(!p.chips.is_empty());
        assert!(p.chips.iter().any(|c| c.endpoint.as_ref().and_then(|e| e.get("searchEndpoint")).is_some()));
        let card = p.sections.iter().find(|s| s.kind == SectionKind::Card).expect("top result");
        assert_eq!(card.items[0].title, "KANA-BOON");
        assert_eq!(card.items[0].kind, ItemKind::Artist);
        // The per-result item sections merge into one list.
        assert_eq!(p.sections.iter().filter(|s| s.kind == SectionKind::List).count(), 1);
    }

    #[test]
    fn filtered_search() {
        let p = page(&fixture("searchSongs"));
        let songs = &p.sections[0];
        assert_eq!(songs.title, "Songs");
        assert!(songs.items.iter().all(|i| i.kind == ItemKind::Song));
    }

    #[test]
    fn search_suggestions() {
        let s = suggestions(&fixture("suggest"));
        assert!(!s.queries.is_empty());
        assert!(!s.items.is_empty());
    }

    #[test]
    fn item_menus() {
        let p = page(&fixture("album"));
        let track = &p.sections[0].items[0];
        assert!(track.menu.iter().any(|m| matches!(m.action, MenuAction::Queue { next: true, .. })));
        assert!(track.menu.iter().any(|m| matches!(m.action, MenuAction::Queue { next: false, .. })));
        assert!(track.menu.iter().any(|m| matches!(m.action, MenuAction::AddToPlaylist { video_id: Some(_), .. })));
        assert!(track.menu.iter().any(|m| matches!(m.action, MenuAction::Share)));
        let library = track.menu.iter().find(|m| m.icon == "BOOKMARK_BORDER").expect("save to library");
        assert!(matches!(&library.action, MenuAction::Service { endpoint } if endpoint.get("feedbackEndpoint").is_some()));
        assert!(library.toggle.is_some());
        assert_eq!(track.like.as_ref().unwrap().video_id, track.video_id.clone().unwrap());
        let h = p.header.unwrap();
        assert!(h.save.is_some());
        assert!(h.menu.iter().any(|m| matches!(m.action, MenuAction::Queue { playlist_id: Some(_), .. })));
    }

    #[test]
    fn endpoints_are_trimmed() {
        let e = endpoint(Some(&json!({
            "clickTrackingParams": "x",
            "browseEndpoint": { "browseId": "MPREb_1", "browseEndpointContextSupportedConfigs": { "browseEndpointContextMusicConfig": { "pageType": "MUSIC_PAGE_TYPE_ALBUM" } } }
        })))
        .unwrap();
        assert_eq!(e, json!({ "browseEndpoint": { "browseId": "MPREb_1", "pageType": "MUSIC_PAGE_TYPE_ALBUM" } }));
    }
}
