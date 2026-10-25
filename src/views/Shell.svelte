<script lang="ts">
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { listen } from "@tauri-apps/api/event";
  import { onMount } from "svelte";
  import { fade, fly } from "svelte/transition";
  import { cubicOut } from "svelte/easing";
  import Icon from "../components/Icon.svelte";
  import NowPlaying from "../components/NowPlaying.svelte";
  import PlayerBar from "../components/PlayerBar.svelte";
  import Tour from "../components/Tour.svelte";
  import Browser from "../components/browse/Browser.svelte";
  import UpNext from "../components/browse/UpNext.svelte";
  import { auth, nowPlaying } from "../lib/music.svelte";
  import { keyboardControls } from "../lib/keyboard";
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
  let tintedBar = $state(true);
  let nativeBrowse = $state(true);
  let upNext = $state(true);
  let showVideos = $state(true);

  const clock = new ProgressClock();
  const lyrics = new LyricsTrack();
  const nowPlayingOpen = $derived(memory?.nowPlayingOpen ?? false);
  // The app draws YouTube Music's pages itself; the YTM page only plays and shows its
  // player page over them.
  const native = $derived(modernUi && nativeBrowse);
  // YTM's header carries the window controls whenever it is on screen: one header, not two.
  // It stays in fullscreen, where its window buttons give way to an exit button.
  const tourOpen = $derived(memory?.tourOpen ?? false);
  // The tour also hides the YTM view, and points at controls in this title bar.
  // The now playing view carries the window controls in its own header instead.
  const showTitleBar = $derived(!native && (tourOpen || !memory?.ytmOwnsHeader));
  // A fullscreen window has nothing to drag or maximize.
  const drag = $derived(fullscreen ? undefined : true);

  const video = $derived(player?.video ?? null);
  const playing = $derived(player?.player.trackState === 1);
  const progress = $derived(video && video.durationSeconds > 0 ? (player!.player.videoProgress / video.durationSeconds) * 100 : 0);
  const art = $derived(bestThumbnail(video?.thumbnails, 60));
  const elapsed = $derived(clock.elapsed(playing, video?.durationSeconds ?? 0));
  const lineIndex = $derived(lyrics.lineAt(elapsed));

  function applySettings(s: Settings) {
    devTools = s.developer.enableDevTools;
    modernUi = s.appearance.modernUi;
    tintedBar = s.appearance.tintedPlayerBar;
    nativeBrowse = s.appearance.nativeBrowse;
    upNext = s.appearance.upNext;
    showVideos = s.playback.showVideos;
  }

  $effect(() => {
    clock.sync(player?.player.videoProgress ?? 0);
  });

  // The clock only needs frames while time advances.
  $effect(() => {
    if (playing) return clock.start();
  });

  $effect(() => {
    nowPlaying.id = video?.id ?? "";
    nowPlaying.playing = playing;
  });

  // Leaving a sign-in page back to YouTube Music changes the session the pages use.
  let wasForeign = false;
  $effect(() => {
    const foreign = memory?.ytmForeignPage ?? false;
    if (wasForeign && !foreign) auth.session++;
    wasForeign = foreign;
  });

  // Lyrics are only fetched while something shows them.
  $effect(() => {
    if (video?.id && nowPlayingOpen) lyrics.load(video.id);
  });


  function onKey(e: KeyboardEvent) {
    if (e.key === "Escape" && native && memory?.videoView === "open") {
      api.command("togglePlayerPage");
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
    let onControlKey: ((e: KeyboardEvent) => void) | undefined;
    keyboardControls().then((handler) => window.addEventListener("keydown", (onControlKey = handler)));
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
      if (onControlKey) window.removeEventListener("keydown", onControlKey);
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
      <div class="nav" data-tour="nav">
        <button class="icon-btn" title="Back" onclick={() => api.ytmHistory(false)}><Icon name="back" size={18} /></button>
        <button class="icon-btn" title="Forward" onclick={() => api.ytmHistory(true)}><Icon name="forward" size={18} /></button>
        <button class="icon-btn" title="Home" onclick={() => api.ytmNavigateHome()}><Icon name="home" size={18} /></button>
      </div>
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
      {@render windowControls()}
    </div>
  </header>
  {/if}

  {#snippet windowControls()}
      {#if devTools}
        <button class="icon-btn" title="YouTube Music developer tools" onclick={() => api.openYtmDevtools()}><Icon name="code" size={18} /></button>
      {/if}
      <button class="menu-btn" data-tour="plugins" title="Plugins" onclick={() => api.showPluginsMenu()}><Icon name="extension" size={16} />Plugins</button>
      <button class="icon-btn" data-tour="mini" title="Mini player" onclick={() => api.toggleMiniPlayer()}><Icon name="mini" size={18} /></button>
      <button class="icon-btn" data-tour="settings" title="Settings" onclick={() => api.openSettings()}><Icon name="settings" size={18} /></button>
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
  {/snippet}

  {#if native}
    <!-- Kept mounted under the lyrics view, so pages keep their place. -->
    <div class="native">
      <Browser controls={windowControls} {drag} {player} videoOpen={memory?.videoView === "open"} signingIn={memory?.ytmForeignPage ?? false} />
      {#if upNext}<UpNext {player} {elapsed} />{/if}
      {#if nowPlayingOpen}
        <div class="layer overlay" in:fly={{ y: 28, duration: 260, easing: cubicOut }} out:fade={{ duration: 140 }}>
          <NowPlaying
            {player}
            {lyrics}
            {lineIndex}
            {elapsed}
            controls={windowControls}
            {drag}
            onclose={() => api.setNowPlayingOpen(false)}
            onseek={(t) => {
              clock.sync(t);
              api.command("seekTo", t);
            }}
          />
        </div>
      {/if}
    </div>
  {:else if nowPlayingOpen}
    <div class="layer" in:fly={{ y: 28, duration: 260, easing: cubicOut }} out:fade={{ duration: 140 }}>
    <NowPlaying
      {player}
      {lyrics}
      {lineIndex}
      {elapsed}
      controls={showTitleBar ? undefined : windowControls}
      {drag}
      onclose={() => api.setNowPlayingOpen(false)}
      onseek={(t) => {
        clock.sync(t);
        api.command("seekTo", t);
      }}
    />
    </div>
  {:else if memory?.ytmViewLoading !== false}
    <!-- A skeleton of YTM's layout, so the app looks ready from the first frame while the page
         loads behind it. -->
    <main class="loader" out:fade={{ duration: 180 }}>
      {#if !memory?.ytmViewLoadTimedout}
        <div class="skeleton" aria-hidden="true">
          <div class="sk-nav"><span class="sk sk-logo"></span><span class="sk sk-search"></span></div>
          <div class="sk-body">
            <div class="sk-side">
              {#each [0, 1, 2] as i (i)}<span class="sk sk-entry"></span>{/each}
              <span class="sk-gap"></span>
              {#each [0, 1, 2, 3, 4] as i (i)}<span class="sk sk-entry short"></span>{/each}
            </div>
            <div class="sk-main">
              <div class="sk-chips">{#each [0, 1, 2, 3, 4, 5] as i (i)}<span class="sk sk-chip"></span>{/each}</div>
              <span class="sk sk-heading"></span>
              <div class="sk-row">{#each [0, 1, 2, 3, 4, 5] as i (i)}<span class="sk sk-card"></span>{/each}</div>
              <span class="sk sk-heading"></span>
              <div class="sk-row">{#each [0, 1, 2, 3, 4, 5] as i (i)}<span class="sk sk-card"></span>{/each}</div>
            </div>
          </div>
        </div>
      {/if}
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
    <PlayerBar {player} {memory} {clock} {showVideos} {fullscreen} tinted={tintedBar} {native} />
  {/if}

  {#if tourOpen}
    <Tour onclose={() => api.setTourOpen(false)} />
  {/if}
</div>

<style>
  .shell { height: 100%; display: flex; flex-direction: column; background: var(--bg); }

  .titlebar {
    height: 40px;
    flex: none;
    display: grid;
    grid-template-columns: 1fr minmax(0, 420px) 1fr;
    align-items: center;
    gap: 12px;
    padding: 0 0 0 10px;
    background: var(--bg);
  }

  .left, .right, .nav { display: flex; align-items: center; gap: 2px; }
  .right { justify-content: flex-end; }
  .left .icon-btn, .right .icon-btn { width: 30px; height: 30px; }

  .brand { display: flex; align-items: center; gap: 8px; margin-right: 8px; }
  .logo {
    width: 20px; height: 20px; border-radius: 6px;
    display: grid; place-items: center;
    background: var(--accent-gradient); color: #fff;
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
  .bar { position: absolute; left: 0; bottom: 0; height: 2px; background: var(--accent-gradient); }

  .menu-btn {
    display: flex; align-items: center; gap: 6px;
    height: 30px; padding: 0 14px; margin-right: 6px;
    border-radius: 999px;
    background: var(--bg-highlight);
    color: var(--text); font-size: 12.5px; font-weight: 700;
    transition: background var(--dur-fast) var(--ease), transform var(--dur-fast) var(--ease);
  }
  .menu-btn:hover { background: #3a3a3a; transform: scale(1.04); }
  .menu-btn:active { transform: scale(0.97); }
  .divider { width: 1px; height: 18px; background: var(--border); margin: 0 6px; }
  .win-btn {
    width: 46px; height: 40px;
    display: grid; place-items: center;
    color: var(--text-muted);
    transition: background var(--dur-fast) var(--ease), color var(--dur-fast) var(--ease);
  }
  .win-btn:hover { background: var(--surface-hover); color: var(--text); }
  .win-btn.close:hover { background: #c42b1c; color: #fff; }

  .ytm-slot { flex: 1; }
  .native { position: relative; flex: 1; min-height: 0; display: flex; flex-direction: column; }
  .overlay { position: absolute; inset: 0; z-index: 10; }
  .layer { flex: 1; min-height: 0; display: flex; }
  .loader {
    flex: 1; min-height: 0;
    position: relative;
    display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 16px;
  }
  .skeleton { position: absolute; inset: 0; display: flex; flex-direction: column; pointer-events: none; }
  .sk { position: relative; overflow: hidden; display: block; border-radius: 6px; background: rgba(255, 255, 255, 0.05); }
  .sk::after {
    content: ""; position: absolute; inset: 0;
    background: linear-gradient(90deg, transparent, rgba(255, 255, 255, 0.06), transparent);
    transform: translateX(-100%);
    animation: shimmer 1.3s linear infinite;
  }
  @keyframes shimmer { to { transform: translateX(100%); } }
  .sk-nav { height: 64px; flex: none; display: flex; align-items: center; gap: 40px; padding: 0 24px; }
  .sk-logo { width: 96px; height: 24px; }
  .sk-search { width: min(480px, 40%); height: 48px; border-radius: 999px; }
  .sk-body { flex: 1; min-height: 0; display: flex; gap: var(--gutter); padding: 0 var(--gutter) var(--gutter); }
  .sk-side { width: 240px; flex: none; display: flex; flex-direction: column; gap: 10px; padding: 16px 12px; border-radius: var(--radius-panel); background: var(--bg-panel); }
  .sk-entry { height: 36px; }
  .sk-entry.short { height: 44px; }
  .sk-gap { height: 12px; }
  .sk-main { flex: 1; min-width: 0; padding: 20px 32px; border-radius: var(--radius-panel); background: var(--bg-panel); display: flex; flex-direction: column; gap: 20px; overflow: hidden; }
  .sk-chips { display: flex; gap: 10px; }
  .sk-chip { width: 84px; height: 32px; border-radius: 999px; }
  .sk-heading { width: 220px; height: 26px; }
  .sk-row { display: flex; gap: 20px; }
  .sk-card { width: 180px; aspect-ratio: 1; flex: none; border-radius: 8px; }
  .status { position: relative; margin: auto 0 18px; color: var(--text-faint); font-size: 12px; }
  @media (max-width: 860px) { .sk-side { display: none; } }
  .timeout { text-align: center; }
  .timeout p { display: flex; gap: 8px; align-items: center; color: var(--warning); margin: 0 0 14px; }
  .actions { display: flex; gap: 10px; justify-content: center; }

</style>
