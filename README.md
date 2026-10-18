# YTMusic Redux

By **Mirang Bhandari**. A fast, lightweight YouTube Music desktop client built with **Tauri 2 + Rust**
and a **Svelte 5** UI. It reimplements
[YouTube Music Desktop App](https://github.com/ytmdesktop/ytmdesktop) (Electron), feature for feature,
and adds a mini player and a redesigned interface.

> Free and open-source software under the GNU GPL-3.0. Not affiliated with Google or YouTube.

## Features

| Upstream feature | Here |
| --- | --- |
| YTM view with custom title bar | Frameless window: Svelte title bar webview + YTM child webview (`src-tauri/src/windows.rs`) |
| Player state store (track, queue, likes, volume, ads) | `src-tauri/src/player.rs`, fed by the injected bridge (`src-tauri/src/ytm/bridge.js`) |
| Remote control (play/pause, seek, volume, repeat, shuffle, queue, like/dislike, navigate) | `ytm::Command` → `window.__YTMR__.exec` |
| Back/forward arrows, library + add-to-playlist buttons, sleep timer, hidden cast button | Injected into YTM by the bridge |
| Continue where you left off (+ pause on launch) | Last URL/video/playlist in `state.json`; autoplay policy browser arg |
| Navigation allow-list, external links to browser | `ytm::is_allowed_navigation`, `on_new_window` |
| Loading screen, 30 s timeout, retry / load home | Shell view + memory store |
| Companion server (REST v1 + Socket.IO realtime, port 9863) | `integrations/companion.rs` (axum + socketioxide), same routes, payloads, error codes and rate limits |
| Companion authorization (4-digit code, 5-min window, token management) | Auth window + Settings → Integrations |
| Discord Rich Presence | Native Discord IPC client (`integrations/discord.rs`) |
| Last.fm scrobbling | `integrations/lastfm.rs`; session key in the OS credential store |
| Song-change notifications | `integrations/notifications.rs` |
| Custom CSS with live reload | `integrations/custom_css.rs` (file watcher) |
| Ratio (cubic) volume | Bridge `ratioVolume` |
| Global shortcuts (7 actions) with failure reporting | `shortcuts.rs` |
| Tray (show/hide, play/pause, prev/next, quit) + icon style | `tray.rs` (+ mini player, settings) |
| Taskbar progress + Windows thumbnail toolbar buttons | `taskbar.rs` (ITaskbarList3) |
| Hide to tray, start on boot, start minimized, zoom, always-show volume slider | Settings |
| Disable hardware acceleration, speaker fill | WebView2 browser args (restart required) |
| Single instance + deep links | `ytmr://play/<videoId>[/<playlistId>]` |
| **From [ytmdesktop2](https://github.com/Venipa/ytmdesktop2):** shared links | Swap `https://` for `ytmr://` on a YouTube Music, YouTube or youtu.be track, playlist, channel or `@handle` URL to open it in the app (`ytm::shared_link`) |
| **From ytmdesktop2:** performance fixes | Bundled CPU tamer and rm3 patches cut YTM's background CPU and memory use (`src-tauri/src/ytm/vendor`) |
| **From ytmdesktop2:** skip disliked songs, music video quality, toast auto-close | Settings → Audio; YTM's "Saved to…" toasts close after 3 s |
| **New:** Modern UI (default on, Settings → Appearance) | App-rendered bottom player bar with a Samsung Music-style **wave seek line** (the played part ripples while playing, flat when paused), glass theme for the YTM page, artwork-tinted ambient background, sleep timer with countdown |
| **New:** mini player | Always-on-top player with the same wave seek line, like, volume (scroll on art) |

## Why Redux

YTM Desktop is the best-known YouTube Music client, but its issue tracker shows the same
complaints over and over: a heavy download, high CPU use, and long-standing feature requests that
are still open. YTMusic Redux keeps every upstream feature (see [Features](#features)), fixes those
pain points, and ships at **about 1/30 of the size**.

### Lightweight

| | YTMusic Redux 1.0.0 | YTM Desktop 2.0.12 |
| --- | --- | --- |
| Windows installer | **5.2 MB** (MSI) | 157.3 MB (`Setup.exe`) |
| Browser engine | Windows' own WebView2, shared with the OS | A private copy of Chromium in every install |
| Backend | Native Rust | Node.js |

Sizes are from this repository's release build and upstream's latest GitHub release.

Upstream users have reported
[very high CPU use while nothing plays](https://github.com/ytmdesktop/ytmdesktop/issues/359),
[high CPU on macOS](https://github.com/ytmdesktop/ytmdesktop/issues/125),
[an app slow enough to slow down the whole system](https://github.com/ytmdesktop/ytmdesktop/issues/249),
[a memory leak](https://github.com/ytmdesktop/ytmdesktop/issues/699) and
[a UI that is slow and laggy, with tracks stopping or skipping](https://github.com/ytmdesktop/ytmdesktop/issues/1570).
Redux avoids most of that overhead by design:

- No Node.js process and no bundled Chromium. The backend (player state, companion server, Discord
  IPC, Last.fm, tray, shortcuts, taskbar) is native Rust.
- Idle costs nothing: the seek-bar animation stops when playback pauses, and lyrics are fetched
  only while something shows them.
- Turning videos off switches YouTube Music to the audio stream, so it also saves bandwidth.

The YouTube Music page is still the biggest consumer of memory in both apps. Redux removes the
runtime around it.

### Features upstream users are still asking for

| Upstream request | In Redux |
| --- | --- |
| [Ability to turn off video](https://github.com/ytmdesktop/ytmdesktop/issues/46) (open since the early days, 30+ comments) | One-click audio-only mode. The player shows the track's artwork instead of a blank area |
| [Realtime lyrics](https://github.com/ytmdesktop/ytmdesktop/issues/809), [lyrics in the same window](https://github.com/ytmdesktop/ytmdesktop/issues/1005) | Synced lyrics in a built-in Now Playing view (LRCLIB). Click a line to seek. Optional lyric captions over the player |
| [AdBlock support](https://github.com/ytmdesktop/ytmdesktop/issues/114) | Built-in ad blocking (Settings → Audio): ads are removed from YouTube Music's responses before they load, and any that still play are skipped |
| [Equalizer](https://github.com/ytmdesktop/ytmdesktop/issues/819) | 10-band equalizer in Settings → Audio |
| [Crossfade](https://github.com/ytmdesktop/ytmdesktop/issues/391) | Fade out the end of each track and fade in the next (0–12 s) |
| [Mini player for v2](https://github.com/ytmdesktop/ytmdesktop/issues/1251) | Always-on-top mini player with seek, like and volume controls |
| [Sleep timer](https://github.com/ytmdesktop/ytmdesktop/issues/451) | Sleep timer in the player bar, with a live countdown |
| — | Playback speed (0.5×–2×) |
| — | Modern UI: an app-rendered player bar that stays visible in fullscreen, and a theme that matches it |

### Security by default

The YouTube Music page can call only the whitelisted `ytm_*` commands, and each command checks
that the caller is the YTM webview (see [Security notes](#security-notes)). Electron gives the
app broad Node.js access, which has to be locked down by hand.

### Trade-offs

Redux is Windows-only for now and has no auto-updater (see
[Differences from upstream](#differences-from-upstream)). Upstream also builds for macOS and
Linux.

## Setup

Prerequisites: Rust (stable), Node 20+, and the [Tauri prerequisites](https://tauri.app/start/prerequisites/) (WebView2 on Windows).

```sh
npm install
npm run tauri dev     # run
npm run tauri build   # installer in src-tauri/target/release/bundle
cd src-tauri && cargo test
```

### Integrations that need your own credentials

The upstream app ships its own Discord application ID and Last.fm API key. This project does not reuse them:

- **Discord**: create an application at <https://discord.com/developers/applications>. Its name is what friends see ("Listening to …"). Paste its ID into Settings → Integrations.
- **Last.fm**: create an API account at <https://www.last.fm/api/account/create>. Enter the key and secret, enable scrobbling, then approve access in the browser.

## License and credits

YTMusic Redux was created and is maintained by **Mirang Bhandari**. It is free and open-source
software under the [GNU General Public License v3.0](LICENSE): you may use, study, share and
modify it, and distributed copies and derivatives must stay under the same license with this
credit kept intact. See [NOTICE](NOTICE) for copyright and attribution.

Contributions are welcome: open an issue or pull request. Contributors keep the copyright to
their contributions, which are licensed under GPL-3.0 like the rest of the project.

Built on [YouTube Music Desktop App](https://github.com/ytmdesktop/ytmdesktop) by the YTMDesktop
team (GPL-3.0); the YouTube Music bridge script is ported from it.

## Code signing

Official releases are signed by the maintainer. `npm run tauri build` signs the app and the MSI through `src-tauri/scripts/sign.ps1` (SHA-256,
DigiCert timestamp). It uses the certificate named by the `YTMR_SIGN_THUMBPRINT` environment
variable, otherwise the author's `CN=Mirang Bhandari` code-signing certificate from the current
user's certificate store. Without a certificate the build stays unsigned. A certificate from a
trusted provider (e.g. Azure Trusted Signing or an OV certificate) removes the SmartScreen
warning; install it and set `YTMR_SIGN_THUMBPRINT`, no code changes needed.

## Security notes

- The YTM page can call only the `ytm_*` bridge commands (`capabilities/ytm.json`, remote URL `https://music.youtube.com/*`). Each command also checks that the caller is the YTM webview.
- Companion tokens are stored only as SHA-256 hashes. The raw token is returned once, to the client.
- The companion server listens on `0.0.0.0:9863` like upstream, so LAN devices can reach it (token required).

## Differences from upstream

- No auto-updater: Tauri's updater needs a signing key and release feed.
- No v1 settings migration, no "page is preventing navigation" prompt, and no renderer-unresponsive detection: Tauri exposes no hooks for these.
