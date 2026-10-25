// Browser preview: answers the app's Tauri calls with sample data so the UI renders in a
// normal browser. Loaded only by preview.html; the app build never includes it.
// URL options: ?view=shell|mini|settings, &queue=1 opens the queue, &lyrics=1 now playing.
import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import type { Memory, PlayerState, QueueItem, Settings } from "./lib/api";
import type { Endpoint, GuideEntry, Item, Page, Section } from "./lib/music.svelte";

const params = new URLSearchParams(location.search);
const pic = (seed: string, size = 544) => [{ url: `https://picsum.photos/seed/${seed}/${size}`, width: size, height: size }];
const run = (text: string) => ({ text, endpoint: null });
const watch = (videoId: string): Endpoint => ({ watchEndpoint: { videoId } });
const browse = (browseId: string): Endpoint => ({ browseEndpoint: { browseId } });

const ARTISTS = ["Nova Lane", "The Midnight Arcade", "Kairo", "Selene Park", "Echo Harbor", "Juno Vale", "Static Bloom", "Aria Monroe"];
const SONGS = ["Neon Tides", "Paper Planes at Dawn", "Gravity Falls Slowly", "Velvet Static", "Afterglow", "City of Glass", "Low Tide", "Northern Lines", "Satellite Hearts", "Golden Hour", "Fade Into You", "Run the Night"];

const item = (over: Partial<Item> & Pick<Item, "kind" | "title">): Item => ({
  subtitle: [],
  thumbnails: pic(over.title),
  endpoint: null,
  play: null,
  videoId: null,
  duration: null,
  index: null,
  aspect: "square",
  color: null,
  menu: [],
  like: null,
  ...over
});
const song = (i: number, index?: number): Item =>
  item({
    kind: "song",
    title: SONGS[i % SONGS.length],
    subtitle: [run(ARTISTS[i % ARTISTS.length]), run(" • "), run(`${(i * 7) % 90 + 10}M plays`)],
    thumbnails: pic(`song${i}`, 120),
    endpoint: watch(`v${i}`),
    videoId: `v${i}`,
    duration: `${2 + (i % 3)}:${String((i * 13) % 60).padStart(2, "0")}`,
    index: index ? String(index) : null,
    like: { videoId: `v${i}`, status: i % 4 === 0 ? "LIKE" : "INDIFFERENT" },
    menu: [{ text: "Play next", icon: "QUEUE_PLAY_NEXT", action: { type: "queue", next: true, videoId: `v${i}`, playlistId: null }, toggle: null }]
  });
const album = (i: number): Item =>
  item({
    kind: "album",
    title: ["Midnight Signals", "Coastlines", "Hollow Sun", "Polaroid Summer", "Blue Hour", "Electric Dreams", "Wildflowers", "Overdrive"][i % 8],
    subtitle: [run("Album"), run(" • "), run(ARTISTS[(i + 2) % ARTISTS.length])],
    thumbnails: pic(`album${i}`),
    endpoint: browse(`MPREb_${i}`),
    play: { watchPlaylistEndpoint: { playlistId: `OLAK${i}` } },
    menu: [{ text: "Save to library", icon: "LIBRARY_ADD", action: { type: "share" }, toggle: null }]
  });
const artist = (i: number): Item =>
  item({ kind: "artist", title: ARTISTS[i % ARTISTS.length], subtitle: [run(`${(i * 3) % 9 + 1}.${i}M subscribers`)], thumbnails: pic(`artist${i}`), endpoint: browse(`UC${i}`), aspect: "circle" });
const video = (i: number): Item =>
  item({ kind: "video", title: SONGS[(i + 5) % SONGS.length] + " (Official Video)", subtitle: [run(ARTISTS[i % 8]), run(" • "), run("24M views")], thumbnails: [{ url: `https://picsum.photos/seed/vid${i}/640/360`, width: 640, height: 360 }], endpoint: watch(`w${i}`), videoId: `w${i}`, aspect: "wide" });
const COLORS = ["#e13300", "#1e3264", "#8d67ab", "#148a08", "#e8115b", "#bc5900", "#477d95", "#503750"];
const category = (i: number): Item =>
  item({ kind: "category", title: ["Chill", "Workout", "Focus", "Party", "Romance", "Sleep", "Commute", "Feel good"][i], color: COLORS[i], endpoint: browse(`FEmusic_moods_${i}`) });
const range = (n: number, f: (i: number) => Item) => Array.from({ length: n }, (_, i) => f(i));
const section = (over: Partial<Section> & Pick<Section, "kind" | "title" | "items">): Section => ({ strapline: null, more: null, text: null, rows: false, continuation: null, ...over });

function page(endpoint: Endpoint): Page {
  const id = endpoint.browseEndpoint?.browseId ?? "";
  if (id.startsWith("VL") || id.startsWith("MPRE")) {
    const playlist = id.startsWith("VL");
    return {
      header: {
        kind: "collection",
        title: playlist ? "Late Night Drive" : "Midnight Signals",
        subtitle: [run(playlist ? "Playlist" : "Album"), run(" • "), run(playlist ? "You" : "Nova Lane"), run(" • "), run("2025")],
        secondSubtitle: [run("12 songs • 41 minutes")],
        description: "Synth-soaked tracks for empty highways and city lights after dark.",
        thumbnails: pic(id),
        play: watch("v0"),
        shuffle: watch("v3"),
        radio: playlist ? null : watch("v5"),
        save: { text: "Save to library", icon: "LIBRARY_ADD", action: { type: "share" }, toggle: null },
        menu: [{ text: "Share", icon: "SHARE", action: { type: "share" }, toggle: null }]
      },
      chips: [],
      sections: [section({ kind: "list", title: "", items: range(12, (i) => song(i, playlist ? undefined : i + 1)) }), section({ kind: "carousel", title: "More from Nova Lane", items: range(8, album) })],
      continuation: null
    };
  }
  if (id.startsWith("UC")) {
    return {
      header: { kind: "artist", title: ARTISTS[Number(id.slice(2)) % 8], subtitle: [run("8.2M monthly audience")], secondSubtitle: [], description: "Genre-blending producer and songwriter.", thumbnails: [{ url: `https://picsum.photos/seed/${id}/1600/700`, width: 1600, height: 700 }], play: watch("v1"), shuffle: watch("v2"), radio: watch("v4"), save: { text: "Subscribe", icon: "BOOKMARK_BORDER", action: { type: "share" }, toggle: null }, menu: [] },
      chips: [],
      sections: [section({ kind: "list", title: "Top songs", items: range(5, (i) => song(i + 2)), more: browse("VLtop") }), section({ kind: "carousel", title: "Albums", items: range(8, album) }), section({ kind: "carousel", title: "Fans might also like", items: range(8, (i) => artist(i + 1)) })],
      continuation: null
    };
  }
  if (id === "FEmusic_explore") {
    return {
      header: null,
      chips: [],
      sections: [section({ kind: "grid", title: "Moods & genres", items: range(8, category) }), section({ kind: "carousel", title: "New albums & singles", items: range(10, (i) => album(i + 3)), more: browse("x") }), section({ kind: "carousel", title: "Trending videos", items: range(8, video) })],
      continuation: null
    };
  }
  if (id === "FEmusic_library_landing") {
    return { header: { kind: "plain", title: "Library", subtitle: [], secondSubtitle: [], description: null, thumbnails: [], play: null, shuffle: null, radio: null, save: null, menu: [] }, chips: [{ text: "Playlists", endpoint: null, selected: true }, { text: "Songs", endpoint: null, selected: false }, { text: "Albums", endpoint: null, selected: false }, { text: "Artists", endpoint: null, selected: false }], sections: [section({ kind: "grid", title: "", items: range(12, album) })], continuation: null };
  }
  return {
    header: null,
    chips: ["Energize", "Relax", "Workout", "Focus", "Commute", "Party", "Romance", "Sad", "Feel good", "Sleep"].map((text) => ({ text, endpoint: null, selected: false })),
    sections: [
      section({ kind: "carousel", title: "Quick picks", strapline: "Start radio from a song", rows: true, items: range(16, song) }),
      section({ kind: "carousel", title: "Listen again", items: range(10, album) }),
      section({ kind: "carousel", title: "Recommended artists", items: range(8, artist) }),
      section({ kind: "carousel", title: "Music videos for you", items: range(8, video) }),
      section({ kind: "carousel", title: "Moods & moments", items: range(8, category) })
    ],
    continuation: null
  };
}

const guide: GuideEntry[] = ["Liked music", "Late Night Drive", "Focus Flow", "Gym Bangers", "Sunday Morning", "Road Trip 2025", "Indie Discoveries", "Throwbacks", "Lo-fi Study", "Rainy Days"].map((title, i) => ({
  title,
  subtitle: i === 0 ? "Auto playlist" : "You",
  endpoint: browse(`VL${i}`),
  play: { watchPlaylistEndpoint: { playlistId: `PL${i}` } },
  // One without a cover shows the fallback tile.
  thumbnails: i === 6 ? [] : pic(`playlist${i}`, 120)
}));

const queueItem = (i: number): QueueItem => ({ thumbnails: pic(`song${i}`, 120), title: SONGS[i % 12], author: ARTISTS[i % 8], videoId: `v${i}`, duration: `3:${String((i * 13) % 60).padStart(2, "0")}`, selected: i === 2 });
let player: PlayerState = {
  player: { trackState: 1, videoProgress: 74, volume: 68, muted: false, adPlaying: false, queue: { repeatMode: 1, items: range(8, (i) => i as never).map((_, i) => queueItem(i)), automixItems: range(5, (i) => i as never).map((_, i) => queueItem(i + 8)), selectedItemIndex: 2 } },
  video: { author: "Kairo", title: "Gravity Falls Slowly", album: "Hollow Sun", likeStatus: 2, thumbnails: pic("song2"), durationSeconds: 214, id: "v2", isLive: false },
  playlistId: null
};

const memory: Memory = {
  discordPresenceConnectionFailed: false,
  shortcutRegisterFailed: {},
  companionServerAuthWindowEnabled: false,
  companionServerError: null,
  lastfmStatus: null,
  secureStorageAvailable: true,
  ytmViewLoading: false,
  ytmViewLoadTimedout: false,
  ytmViewLoadingStatus: "",
  restartRequired: false,
  sleepTimerEndsAt: null,
  nowPlayingOpen: params.has("lyrics"),
  ytmOwnsHeader: false,
  tourOpen: false,
  ytmPlayerPage: false,
  ytmForeignPage: false,
  videoView: "closed",
  smartAudio: "tuned"
};

const settings: Settings = {
  general: { disableHardwareAcceleration: false, hideToTrayOnClose: false, showNotificationOnSongChange: false, startOnBoot: false, startMinimized: false },
  appearance: { modernUi: true, tintedPlayerBar: true, nativeBrowse: true, videoMiniPlayer: true, upNext: true, alwaysShowVolumeSlider: false, customCssEnabled: false, customCssPath: null, zoom: 100, trayIconStyle: "auto" },
  playback: { continueWhereYouLeftOff: true, continueWhereYouLeftOffPaused: true, enableSpeakerFill: false, progressInTaskbar: false, ratioVolume: false, showVideos: true, openPlayerOnPlay: false, playbackRate: 100, crossfadeSeconds: 4, equalizer: { enabled: true, gains: [6, 5, 4, 2, 0, 0, 0, 0, 0, 0] }, volumeLeveler: true, stereoWidth: 120, smartAudio: false, blockAds: true, skipDisliked: false, videoQuality: "auto" },
  integrations: { companionServerEnabled: false, companionServerCORSWildcardEnabled: false, discordPresenceEnabled: false, discordClientId: "", lastFMEnabled: false },
  shortcuts: { playPause: "", next: "", previous: "", thumbsUp: "", thumbsDown: "", volumeUp: "", volumeDown: "" },
  lastfm: { apiKey: "", secret: "", scrobblePercent: 50 },
  developer: { enableDevTools: false }
};

// Writes "a.b.c" into the settings object, as the backend does.
function setPath(obj: Record<string, unknown>, key: string, value: unknown) {
  const parts = key.split(".");
  let target = obj as Record<string, unknown>;
  for (const p of parts.slice(0, -1)) target = target[p] as Record<string, unknown>;
  target[parts[parts.length - 1]] = value;
}

mockWindows(params.get("view") ?? "main");
mockIPC(
  (cmd, args) => {
    const a = (args ?? {}) as Record<string, unknown>;
    switch (cmd) {
      case "get_settings":
        return structuredClone(settings);
      case "set_setting":
        setPath(settings as unknown as Record<string, unknown>, a.key as string, a.value);
        return null;
      case "get_memory":
        return memory;
      case "get_player_state":
        return player;
      case "player_command":
        if (a.command === "setVolume") player = { ...player, player: { ...player.player, volume: a.data as number } };
        if (a.command === "playPause") player = { ...player, player: { ...player.player, trackState: player.player.trackState === 1 ? 0 : 1 } };
        return null;
      case "music_browse":
        return page(a.endpoint as Endpoint);
      case "music_search":
        return { header: null, chips: [{ text: "Songs", endpoint: null, selected: false }, { text: "Albums", endpoint: null, selected: false }], sections: [section({ kind: "card", title: "Top result", items: [artist(2), ...range(4, song)] }), section({ kind: "carousel", title: "Albums", items: range(8, album) })], continuation: null };
      case "music_suggestions":
        return { queries: [[run(String(a.input)), run(" remix")], [run(String(a.input)), run(" live")]], items: [artist(1), song(3)] };
      case "music_guide":
        return guide;
      case "music_account":
        return { name: "Preview User", handle: "@preview", photo: [] };
      case "keyboard_bindings":
      case "companion_list":
        return [];
      case "app_info":
        return { version: "preview", tauri: "2" };
      case "lastfm_signed_in":
        return false;
      case "get_lyrics":
        return { videoId: "v2", synced: SONGS.map((text, i) => ({ time: i * 6, text })), plain: null, instrumental: false };
      default:
        return null;
    }
  },
  { shouldMockEvents: true }
);

if (params.has("queue")) import("./lib/music.svelte").then(({ ui }) => (ui.queueOpen = true));
import("./main");
