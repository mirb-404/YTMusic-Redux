import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export type Thumbnail = { url: string; width: number; height: number };

export type PlayerState = {
  player: {
    trackState: -1 | 0 | 1 | 2;
    videoProgress: number;
    volume: number;
    muted: boolean;
    adPlaying: boolean;
    queue: { repeatMode: number; items: unknown[] } | null;
  };
  video: {
    author: string;
    title: string;
    album: string | null;
    likeStatus: -1 | 0 | 1 | 2;
    thumbnails: Thumbnail[];
    durationSeconds: number;
    id: string;
    isLive: boolean;
  } | null;
  playlistId: string | null;
};

export type Settings = {
  general: {
    disableHardwareAcceleration: boolean;
    hideToTrayOnClose: boolean;
    showNotificationOnSongChange: boolean;
    startOnBoot: boolean;
    startMinimized: boolean;
  };
  appearance: {
    modernUi: boolean;
    alwaysShowVolumeSlider: boolean;
    customCssEnabled: boolean;
    customCssPath: string | null;
    zoom: number;
    trayIconStyle: "auto" | "white" | "black";
    lyricsCaptions: boolean;
  };
  playback: {
    continueWhereYouLeftOff: boolean;
    continueWhereYouLeftOffPaused: boolean;
    enableSpeakerFill: boolean;
    progressInTaskbar: boolean;
    ratioVolume: boolean;
    showVideos: boolean;
    playbackRate: number;
    crossfadeSeconds: number;
    equalizer: { enabled: boolean; gains: number[] };
  };
  integrations: {
    companionServerEnabled: boolean;
    companionServerCORSWildcardEnabled: boolean;
    discordPresenceEnabled: boolean;
    discordClientId: string;
    lastFMEnabled: boolean;
  };
  shortcuts: Record<ShortcutKey, string>;
  lastfm: { apiKey: string; secret: string; scrobblePercent: number };
  developer: { enableDevTools: boolean };
};

export type ShortcutKey = "playPause" | "next" | "previous" | "thumbsUp" | "thumbsDown" | "volumeUp" | "volumeDown";

export type Memory = {
  discordPresenceConnectionFailed: boolean;
  shortcutRegisterFailed: Partial<Record<ShortcutKey, boolean>>;
  companionServerAuthWindowEnabled: boolean;
  companionServerError: string | null;
  lastfmStatus: string | null;
  secureStorageAvailable: boolean;
  ytmViewLoading: boolean;
  ytmViewLoadTimedout: boolean;
  ytmViewLoadingStatus: string;
  restartRequired: boolean;
  sleepTimerEndsAt: number | null;
  nowPlayingOpen: boolean;
  ytmOwnsHeader: boolean;
};

export type LyricLine = { time: number; text: string };
export type Lyrics = { videoId: string; synced: LyricLine[] | null; plain: string | null; instrumental: boolean };

export type Companion = { id: string; appId: string; appName: string; appVersion: string };

export type PlayerCommand =
  | "playPause" | "next" | "previous" | "toggleLike" | "toggleDislike" | "shuffle"
  | "volumeUp" | "volumeDown" | "mute" | "unmute" | "setVolume" | "seekTo" | "repeatMode"
  | "togglePlayerPage" | "addToPlaylist";

export const api = {
  getSettings: () => invoke<Settings>("get_settings"),
  setSetting: (key: string, value: unknown) => invoke<void>("set_setting", { key, value }),
  resetSettings: (section: keyof Settings) => invoke<void>("reset_settings", { section }),
  getMemory: () => invoke<Memory>("get_memory"),
  getPlayerState: () => invoke<PlayerState>("get_player_state"),
  command: (command: PlayerCommand, data?: unknown) => invoke<void>("player_command", { command, data }),
  sleepTimerSet: (minutes: number) => invoke<void>("sleep_timer_set", { minutes }),
  openSettings: (tab?: string) => invoke<void>("open_settings", { tab }),
  showPluginsMenu: () => invoke<void>("show_plugins_menu"),
  toggleMiniPlayer: () => invoke<void>("toggle_mini_player"),
  toggleFullscreen: () => invoke<void>("toggle_fullscreen"),
  setNowPlayingOpen: (open: boolean) => invoke<void>("set_now_playing_open", { open }),
  getLyrics: () => invoke<Lyrics | null>("get_lyrics"),
  setCaption: (text: string | null) => invoke<void>("ytm_set_caption", { text }),
  showMainWindow: () => invoke<void>("show_main_window"),
  ytmRecreate: () => invoke<void>("ytm_recreate"),
  ytmNavigateHome: () => invoke<void>("ytm_navigate_home"),
  ytmHistory: (forward: boolean) => invoke<void>("ytm_history", { forward }),
  openYtmDevtools: () => invoke<void>("open_ytm_devtools"),
  restartApp: () => invoke<void>("restart_app"),
  quitApp: () => invoke<void>("quit_app"),
  companionList: () => invoke<Companion[]>("companion_list"),
  companionRevoke: (id: string) => invoke<void>("companion_revoke", { id }),
  companionSetAuthEnabled: (enabled: boolean) => invoke<void>("companion_set_auth_enabled", { enabled }),
  companionAuthInfo: () => invoke<{ appName: string; code: string }>("companion_auth_info"),
  companionAuthResult: (allowed: boolean) => invoke<void>("companion_auth_result", { allowed }),
  lastfmSignedIn: () => invoke<boolean>("lastfm_signed_in"),
  lastfmSignOut: () => invoke<void>("lastfm_sign_out"),
  pickCssFile: () => invoke<string | null>("pick_css_file"),
  appInfo: () => invoke<{ version: string; tauri: string }>("app_info")
};

export const onPlayerState = (f: (s: PlayerState) => void) => listen<PlayerState>("player-state", (e) => f(e.payload));
export const onMemory = (f: (m: Memory) => void) => listen<Memory>("memory-changed", (e) => f(e.payload));
export const onSettings = (f: (s: Settings) => void) => listen<Settings>("settings-changed", (e) => f(e.payload));

export function bestThumbnail(thumbnails: Thumbnail[] | undefined, minWidth = 0): string | null {
  if (!thumbnails?.length) return null;
  const sorted = [...thumbnails].sort((a, b) => a.width - b.width);
  return (sorted.find((t) => t.width >= minWidth) ?? sorted[sorted.length - 1]).url;
}

export function formatTime(seconds: number): string {
  if (!Number.isFinite(seconds) || seconds < 0) seconds = 0;
  const s = Math.floor(seconds % 60).toString().padStart(2, "0");
  const m = Math.floor(seconds / 60) % 60;
  const h = Math.floor(seconds / 3600);
  return h > 0 ? `${h}:${m.toString().padStart(2, "0")}:${s}` : `${m}:${s}`;
}
