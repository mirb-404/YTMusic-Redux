<script lang="ts">
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { listen } from "@tauri-apps/api/event";
  import { onMount } from "svelte";
  import Icon from "../components/Icon.svelte";
  import NowPlaying from "../components/NowPlaying.svelte";
  import PlayerBar from "../components/PlayerBar.svelte";
  import { api, bestThumbnail, onMemory, onPlayerState, onSettings, type Memory, type PlayerState, type Settings } from "../lib/api";
  import { LyricsTrack } from "../lib/lyrics.svelte";
  import { ProgressClock } from "../lib/progress.svelte";

  const appWindow = getCurrentWindow();
  let memory = $state<Memory | null>(null);
  let player = $state<PlayerState | null>(null);
  let maximized = $state(false);
  let fullscreen = $state(false);
  let devTools = $state(false);
  let modernUi = $state(true);
  let showVideos = $state(true);
  let captions = $state(false);

  const clock = new ProgressClock();
  const lyrics = new LyricsTrack();
  const nowPlayingOpen = $derived(memory?.nowPlayingOpen ?? false);
  // YTM's header carries the window controls whenever it is on screen: one header, not two.
  // It stays in fullscreen, where its window buttons give way to an exit button.
  const showTitleBar = $derived(nowPlayingOpen || !memory?.ytmOwnsHeader);
  // A fullscreen window has nothing to drag or maximize.
  const drag = $derived(fullscreen ? undefined : true);

  const video = $derived(player?.video ?? null);
  const playing = $derived(player?.player.trackState === 1);
  const progress = $derived(video && video.durationSeconds > 0 ? (player!.player.videoProgress / video.durationSeconds) * 100 : 0);
  const art = $derived(bestThumbnail(video?.thumbnails, 60));
  const elapsed = $derived(clock.elapsed(playing, video?.durationSeconds ?? 0));
  const lineIndex = $derived(lyrics.lineAt(elapsed));
  const caption = $derived.by(() => {
    if (!captions || nowPlayingOpen || player?.player.adPlaying) return null;
    return lyrics.data?.synced?.[lineIndex]?.text || null;
  });

  function applySettings(s: Settings) {
    devTools = s.developer.enableDevTools;
    modernUi = s.appearance.modernUi;
    showVideos = s.playback.showVideos;
    captions = s.appearance.lyricsCaptions;
  }

  $effect(() => {
    clock.sync(player?.player.videoProgress ?? 0);
  });

  // The clock only needs frames while time advances.
  $effect(() => {
    if (playing) return clock.start();
  });

  // Lyrics are only fetched while something shows them.
  $effect(() => {
    if (video?.id && (nowPlayingOpen || captions)) lyrics.load(video.id);
  });

  $effect(() => {
    api.setCaption(caption);
  });

  function onKey(e: KeyboardEvent) {
    if (e.key === "F11") {
      e.preventDefault();
      api.toggleFullscreen();
    } else if (e.key === "Escape" && nowPlayingOpen) {
      api.setNowPlayingOpen(false);
    } else if (e.key === "Escape" && fullscreen) {
      api.toggleFullscreen();
    }
  }

  onMount(() => {
    api.getMemory().then((m) => (memory = m));
    api.getPlayerState().then((s) => (player = s));
    api.getSettings().then(applySettings);
    window.addEventListener("keydown", onKey);
    appWindow.isMaximized().then((m) => (maximized = m));
    const unlisten = [
      onMemory((m) => (memory = m)),
      onPlayerState((s) => (player = s)),
      onSettings(applySettings),
      listen<{ maximized: boolean; fullscreen: boolean }>("window-state", (e) => {
        maximized = e.payload.maximized;
        fullscreen = e.payload.fullscreen;
      })
    ];
    return () => {
      window.removeEventListener("keydown", onKey);
      unlisten.forEach((p) => p.then((f) => f()));
    };
  });
</script>

<div class="shell">
  {#if showTitleBar}
  <header class="titlebar" data-tauri-drag-region={drag}>
    <div class="left">
      <div class="brand" data-tauri-drag-region={drag}>
        <span class="logo"><Icon name="play" size={14} /></span>
        <span class="name" data-tauri-drag-region={drag}>YTMusic</span>
      </div>
      <button class="icon-btn" title="Back" onclick={() => api.ytmHistory(false)}><Icon name="back" size={18} /></button>
      <button class="icon-btn" title="Forward" onclick={() => api.ytmHistory(true)}><Icon name="forward" size={18} /></button>
      <button class="icon-btn" title="Home" onclick={() => api.ytmNavigateHome()}><Icon name="home" size={18} /></button>
    </div>

    <div class="center" data-tauri-drag-region={drag}>
      {#if video && !modernUi}
        <button class="now-playing" title="Open mini player" onclick={() => api.toggleMiniPlayer()}>
          {#if art}<img src={art} alt="" />{/if}
          <span class="meta">
            <span class="title">{video.title}</span>
            <span class="artist">{video.author}</span>
          </span>
          <span class="bar" style:width="{progress}%"></span>
        </button>
      {/if}
    </div>

    <div class="right">
      {#if video && !modernUi}
        <button class="icon-btn" title={playing ? "Pause" : "Play"} onclick={() => api.command("playPause")}>
          <Icon name={playing ? "pause" : "play"} size={18} />
        </button>
      {/if}
      {#if devTools}
        <button class="icon-btn" title="YouTube Music developer tools" onclick={() => api.openYtmDevtools()}><Icon name="code" size={18} /></button>
      {/if}
      <button class="menu-btn" title="Plugins" onclick={() => api.showPluginsMenu()}><Icon name="extension" size={16} />Plugins</button>
      <button class="icon-btn" title="Mini player" onclick={() => api.toggleMiniPlayer()}><Icon name="mini" size={18} /></button>
      <button class="icon-btn" title="Settings" onclick={() => api.openSettings()}><Icon name="settings" size={18} /></button>
      <span class="divider"></span>
      {#if fullscreen}
        <button class="win-btn" title="Exit full screen (F11)" onclick={() => api.toggleFullscreen()}><Icon name="fullscreenExit" size={18} /></button>
      {:else}
        <button class="win-btn" title="Minimize" onclick={() => appWindow.minimize()}><Icon name="minimize" size={14} /></button>
        <button class="win-btn" title={maximized ? "Restore" : "Maximize"} onclick={() => appWindow.toggleMaximize()}>
          <Icon name={maximized ? "restore" : "maximize"} size={13} />
        </button>
      {/if}
      <button class="win-btn close" title="Close" onclick={() => appWindow.close()}><Icon name="close" size={16} /></button>
    </div>
  </header>
  {/if}

  {#if nowPlayingOpen}
    <NowPlaying
      {player}
      {lyrics}
      {lineIndex}
      {captions}
      {showVideos}
      onclose={() => api.setNowPlayingOpen(false)}
      onseek={(t) => {
        clock.sync(t);
        api.command("seekTo", t);
      }}
    />
  {:else if memory?.ytmViewLoading !== false}
    <main class="loader">
      {#if !memory?.ytmViewLoadTimedout}<span class="spinner" aria-hidden="true"></span>{/if}
      <p class="status">{memory?.ytmViewLoadingStatus || "Starting..."}</p>
      {#if memory?.ytmViewLoadTimedout}
        <div class="timeout">
          <p><Icon name="warning" size={16} /> YouTube Music is taking longer than expected to load.</p>
          <div class="actions">
            <button class="btn" onclick={() => api.ytmNavigateHome()}><Icon name="home" size={16} />Load home page</button>
            <button class="btn primary" onclick={() => api.ytmRecreate()}><Icon name="refresh" size={16} />Retry</button>
          </div>
        </div>
      {/if}
    </main>
  {:else}
    <div class="ytm-slot"></div>
  {/if}

  {#if modernUi}
    <PlayerBar {player} {memory} {clock} {showVideos} {fullscreen} />
  {/if}
</div>

<style>
  .shell { height: 100%; display: flex; flex-direction: column; background: #0b0b0f; }

  .titlebar {
    height: 40px;
    flex: none;
    display: grid;
    grid-template-columns: 1fr minmax(0, 420px) 1fr;
    align-items: center;
    gap: 12px;
    padding: 0 0 0 10px;
    background: var(--bg);
    border-bottom: 1px solid var(--border);
  }

  .left, .right { display: flex; align-items: center; gap: 2px; }
  .right { justify-content: flex-end; }
  .left .icon-btn, .right .icon-btn { width: 30px; height: 30px; }

  .brand { display: flex; align-items: center; gap: 8px; margin-right: 8px; }
  .logo {
    width: 20px; height: 20px; border-radius: 6px;
    display: grid; place-items: center;
    background: var(--accent); color: #fff;
  }
  .name { font-weight: 600; font-size: 13px; }

  .center { display: flex; justify-content: center; min-width: 0; height: 100%; align-items: center; }
  .now-playing {
    position: relative;
    display: flex; align-items: center; gap: 8px;
    max-width: 100%; height: 28px;
    padding: 0 10px 0 2px;
    border-radius: 6px;
    background: var(--surface);
    overflow: hidden;
    transition: background 0.12s;
  }
  .now-playing:hover { background: var(--surface-hover); }
  .now-playing img { width: 24px; height: 24px; border-radius: 4px; object-fit: cover; flex: none; }
  .meta { display: flex; gap: 6px; min-width: 0; font-size: 12.5px; white-space: nowrap; }
  .title { font-weight: 600; overflow: hidden; text-overflow: ellipsis; }
  .artist { color: var(--text-muted); overflow: hidden; text-overflow: ellipsis; }
  .artist::before { content: "·"; margin-right: 6px; color: var(--text-faint); }
  .bar { position: absolute; left: 0; bottom: 0; height: 2px; background: var(--accent); }

  .menu-btn {
    display: flex; align-items: center; gap: 6px;
    height: 28px; padding: 0 10px; margin-right: 4px;
    border-radius: 6px;
    color: var(--text-muted); font-size: 12.5px; font-weight: 600;
    transition: background 0.12s, color 0.12s;
  }
  .menu-btn:hover { background: var(--surface-hover); color: var(--text); }
  .divider { width: 1px; height: 18px; background: var(--border); margin: 0 6px; }
  .win-btn {
    width: 46px; height: 40px;
    display: grid; place-items: center;
    color: var(--text-muted);
    transition: background 0.12s, color 0.12s;
  }
  .win-btn:hover { background: var(--surface-hover); color: var(--text); }
  .win-btn.close:hover { background: #c42b1c; color: #fff; }

  .ytm-slot { flex: 1; }
  .loader {
    flex: 1;
    display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 16px;
  }
  .spinner {
    width: 28px; height: 28px; border-radius: 50%;
    border: 3px solid rgba(255, 255, 255, 0.12);
    border-top-color: var(--accent);
    animation: spin 0.8s linear infinite;
  }
  .status { margin: 0; color: var(--text-muted); font-size: 13px; }
  .timeout { text-align: center; }
  .timeout p { display: flex; gap: 8px; align-items: center; color: var(--warning); margin: 0 0 14px; }
  .actions { display: flex; gap: 10px; justify-content: center; }

  @keyframes spin { to { transform: rotate(360deg); } }
</style>
