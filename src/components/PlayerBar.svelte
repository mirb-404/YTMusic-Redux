<script lang="ts">
  import { onMount } from "svelte";
  import { fade, scale } from "svelte/transition";
  import { cubicOut } from "svelte/easing";
  import Icon from "./Icon.svelte";
  import WaveSlider from "./WaveSlider.svelte";
  import { api, bestThumbnail, formatTime, type Memory, type PlayerState } from "../lib/api";
  import type { ProgressClock } from "../lib/progress.svelte";
  import { ui } from "../lib/music.svelte";

  let {
    player,
    memory,
    clock,
    showVideos,
    fullscreen,
    tinted,
    native = false
  }: {
    player: PlayerState | null;
    memory: Memory | null;
    clock: ProgressClock;
    showVideos: boolean;
    fullscreen: boolean;
    tinted: boolean;
    /** The app's own pages are shown: the queue opens beside them. */
    native?: boolean;
  } = $props();

  const video = $derived(player?.video ?? null);
  const playing = $derived(player?.player.trackState === 1);
  const buffering = $derived(player?.player.trackState === 2);
  const duration = $derived(video?.durationSeconds ?? 0);
  const elapsed = $derived(clock.elapsed(playing, duration));
  const art = $derived(bestThumbnail(video?.thumbnails, 120));
  const repeatMode = $derived(player?.player.queue?.repeatMode ?? 0);
  const muted = $derived(player?.player.muted ?? false);
  const adPlaying = $derived(player?.player.adPlaying ?? false);

  let volumeDraft = $state<number | null>(null);
  const volume = $derived(volumeDraft ?? Math.round(player?.player.volume ?? 0));
  let sleepOpen = $state(false);
  let now = $state(Date.now());
  const sleepLeft = $derived(memory?.sleepTimerEndsAt ? Math.max(0, (memory.sleepTimerEndsAt - now) / 1000) : null);

  const nowPlayingOpen = $derived(memory?.nowPlayingOpen ?? false);

  onMount(() => {
    const timer = setInterval(() => (now = Date.now()), 1000);
    const close = (e: MouseEvent) => {
      if (!(e.target as HTMLElement).closest(".sleep")) sleepOpen = false;
    };
    window.addEventListener("pointerdown", close);
    return () => {
      clearInterval(timer);
      window.removeEventListener("pointerdown", close);
    };
  });

  let volumeTimer: ReturnType<typeof setTimeout> | undefined;
  function setVolume(v: number) {
    volumeDraft = v;
    clearTimeout(volumeTimer);
    volumeTimer = setTimeout(async () => {
      await api.command("setVolume", v);
      if (muted && v > 0) await api.command("unmute");
      volumeDraft = null;
    }, 60);
  }

  function wheelVolume(e: WheelEvent) {
    e.preventDefault();
    setVolume(Math.max(0, Math.min(100, volume + (e.deltaY < 0 ? 5 : -5))));
  }

  function seek(seconds: number) {
    clock.sync(seconds);
    api.command("seekTo", seconds);
  }

  function setSleep(minutes: number) {
    sleepOpen = false;
    api.sleepTimerSet(minutes);
  }

  const sleepOptions = [5, 10, 15, 30, 45, 60, 90];
</script>

<footer class="bar" class:tinted>
  <!-- The artwork, blurred, tints the bar in the track's colors. -->
  <!-- Clipped on its own layer, so popups like the sleep timer can rise above the bar. -->
  {#if tinted && art}<div class="tint-clip" aria-hidden="true">{#key art}<img class="tint" src={art} alt="" transition:fade={{ duration: 700 }} />{/key}</div>{/if}
  <section class="track">
    <div class="art-wrap">
      <button class="art" data-tour="art" title="Now playing" disabled={!video} onclick={() => api.command("togglePlayerPage")}>
        {#if art}{#key art}<img src={art} alt="" in:fade={{ duration: 220 }} />{/key}{:else}<Icon name="music" size={22} />{/if}
        <span class="overlay"><Icon name="expand" size={18} /></span>
      </button>
      {#if video}
        <button class="close-track" title="Close track (Ctrl+Shift+X)" onclick={() => api.command("closeTrack")}><Icon name="close" size={12} /></button>
      {/if}
    </div>
    <div class="meta">
      <div class="title" title={video?.title}>
        {#if adPlaying}<span class="ad">Ad</span>{/if}{video?.title ?? "Nothing playing"}
      </div>
      <div class="sub" title={video ? [video.author, video.album].filter(Boolean).join(" · ") : ""}>
        {#if video}{video.author}{#if video.album}<span class="dot">·</span>{video.album}{/if}{:else}Pick something to play{/if}
      </div>
    </div>
    <div class="rate">
      <button class="icon-btn" class:active={video?.likeStatus === 2} title="Like" disabled={!video} onclick={() => api.command("toggleLike")}>
        <Icon name={video?.likeStatus === 2 ? "like" : "likeOutline"} size={18} />
      </button>
      <button class="icon-btn" class:active={video?.likeStatus === 0} title="Dislike" disabled={!video} onclick={() => api.command("toggleDislike")}>
        <Icon name={video?.likeStatus === 0 ? "dislike" : "dislikeOutline"} size={18} />
      </button>
    </div>
  </section>

  <section class="center">
    <div class="controls" data-tour="controls">
      <button class="icon-btn" title="Shuffle" disabled={!video} onclick={() => api.command("shuffle")}><Icon name="shuffle" size={20} /></button>
      <button class="icon-btn big" title="Previous" disabled={!video} onclick={() => api.command("previous")}><Icon name="previous" size={24} /></button>
      <button class="play" title={playing ? "Pause" : "Play"} disabled={!video} onclick={() => api.command("playPause")}>
        <Icon name={playing ? "pause" : "play"} size={22} />
      </button>
      <button class="icon-btn big" title="Next" disabled={!video} onclick={() => api.command("next")}><Icon name="next" size={24} /></button>
      <button
        class="icon-btn"
        class:active={repeatMode > 0}
        title={["Repeat off", "Repeat all", "Repeat one"][repeatMode] ?? "Repeat"}
        disabled={!video}
        onclick={() => api.command("repeatMode", (repeatMode + 1) % 3)}
      >
        <Icon name={repeatMode === 2 ? "repeatOne" : "repeat"} size={20} />
      </button>
    </div>
    <div class="seek" data-tour="seek">
      <span class="time">{formatTime(elapsed)}</span>
      <WaveSlider value={elapsed} max={duration} playing={playing && !buffering} disabled={!video || video.isLive || adPlaying} onseek={seek} />
      <span class="time end">{video?.isLive ? "LIVE" : formatTime(duration)}</span>
    </div>
  </section>

  <section class="extras">
    <button class="icon-btn" title="Add to playlist" disabled={!video} onclick={() => api.command("addToPlaylist")}><Icon name="playlistAdd" size={20} /></button>
    <div class="sleep" data-tour="sleep">
      <button class="icon-btn" class:active={sleepLeft !== null} title="Sleep timer" onclick={() => (sleepOpen = !sleepOpen)}>
        <Icon name="timer" size={20} />
      </button>
      {#if sleepLeft !== null}<span class="countdown">{formatTime(sleepLeft)}</span>{/if}
      {#if sleepOpen}
        <div class="menu" role="menu" transition:scale={{ start: 0.96, duration: 140, easing: cubicOut }}>
          <div class="menu-title">Pause music after</div>
          {#each sleepOptions as m (m)}
            <button role="menuitem" onclick={() => setSleep(m)}>{m < 60 ? `${m} minutes` : `${m / 60} hour${m > 60 ? "s" : ""}`}</button>
          {/each}
          {#if sleepLeft !== null}<button role="menuitem" class="off" onclick={() => setSleep(0)}>Turn off timer</button>{/if}
        </div>
      {/if}
    </div>
    <button class="icon-btn" data-tour="lyrics" class:active={nowPlayingOpen} title="Lyrics" onclick={() => api.setNowPlayingOpen(!nowPlayingOpen)}><Icon name="lyrics" size={20} /></button>
    <button
      class="icon-btn"
      data-tour="video"
      class:active={showVideos}
      title={showVideos ? "Music videos on (switch to audio only)" : "Audio only (switch to music videos)"}
      onclick={() => {
        api.setSetting("playback.showVideos", !showVideos);
        // Over the app's own pages the video only shows on YTM's player page.
        if (native && !showVideos && video && memory?.videoView === "closed") api.command("togglePlayerPage");
      }}
    >
      <Icon name={showVideos ? "video" : "videoOff"} size={20} />
    </button>
    <button
      class="icon-btn"
      data-tour="queue"
      class:active={native && ui.queueOpen}
      title="Queue (Q)"
      disabled={!video}
      onclick={() => (native ? (ui.queueOpen = !ui.queueOpen) : api.command("togglePlayerPage"))}><Icon name="queue" size={20} /></button
    >
    <div class="volume" data-tour="volume" onwheel={wheelVolume}>
      <button class="icon-btn" title={muted ? "Unmute" : "Mute"} onclick={() => api.command(muted ? "unmute" : "mute")}>
        <Icon name={muted || volume === 0 ? "muted" : volume < 50 ? "volumeLow" : "volume"} size={20} />
      </button>
      <input
        type="range"
        min="0"
        max="100"
        value={muted ? 0 : volume}
        aria-label="Volume"
        style:--fill="{muted ? 0 : volume}%"
        oninput={(e) => setVolume(Number(e.currentTarget.value))}
      />
    </div>
    <button class="icon-btn" data-tour="fullscreen" title={fullscreen ? "Exit full screen (F11)" : "Full screen (F11)"} onclick={() => api.toggleFullscreen()}>
      <Icon name={fullscreen ? "fullscreenExit" : "fullscreen"} size={20} />
    </button>
  </section>
</footer>

<style>
  .bar {
    height: 88px;
    flex: none;
    display: grid;
    grid-template-columns: minmax(240px, 1fr) minmax(320px, 560px) minmax(240px, 1fr);
    align-items: center;
    gap: 24px;
    padding: 0 16px;
    position: relative;
    z-index: 30;
    isolation: isolate;
    background: var(--bg);
  }
  .tint-clip { position: absolute; inset: 0; z-index: -1; overflow: hidden; pointer-events: none; }
  /* Slightly see-through over the tint layer below. */
  .bar.tinted { background: color-mix(in srgb, var(--bg) 82%, transparent); }
  .tint {
    position: absolute; inset: -60px; z-index: -1;
    width: calc(100% + 120px); height: calc(100% + 120px);
    object-fit: cover;
    filter: blur(50px) saturate(1.7);
    opacity: 0.28;
    pointer-events: none;
  }
  section { position: relative; min-width: 0; }

  .track { display: flex; align-items: center; gap: 12px; }
  .art-wrap { position: relative; flex: none; }
  .art {
    position: relative;
    width: 56px; height: 56px; flex: none;
    border-radius: 4px; overflow: hidden;
    display: grid; place-items: center;
    background: var(--bg-highlight);
    color: var(--text-faint);
    box-shadow: 0 4px 14px rgba(0, 0, 0, 0.5);
    transition: transform var(--dur) var(--ease);
  }
  .art:not(:disabled):hover { transform: scale(1.04); }
  .art img { animation: art-in var(--dur-slow) var(--ease-out); }
  @keyframes art-in { from { opacity: 0; transform: scale(0.92); } }
  .art img { width: 100%; height: 100%; object-fit: cover; }
  .overlay {
    position: absolute; inset: 0;
    display: grid; place-items: center;
    background: rgba(0, 0, 0, 0.5); color: #fff;
    opacity: 0; transition: opacity var(--dur) var(--ease);
  }
  .art:not(:disabled):hover .overlay { opacity: 1; }
  .close-track {
    position: absolute; top: -6px; right: -6px; z-index: 1;
    width: 20px; height: 20px; border-radius: 50%;
    display: grid; place-items: center;
    background: var(--bg-elevated); color: var(--text-muted);
    border: 1px solid var(--border);
    box-shadow: 0 2px 6px rgba(0, 0, 0, 0.4);
    opacity: 0; transform: scale(0.8);
    transition: opacity 0.15s var(--ease), transform 0.15s var(--ease), color 0.15s, background 0.15s;
  }
  .track:hover .close-track, .close-track:focus-visible { opacity: 1; transform: none; }
  .close-track:hover { color: #fff; background: var(--danger); border-color: transparent; }
  .meta { min-width: 0; flex: 0 1 auto; }
  .title { font-weight: 500; font-size: 14px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .sub { color: var(--text-muted); font-size: 12px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; margin-top: 2px; }
  .dot { margin: 0 5px; color: var(--text-faint); }
  .ad {
    display: inline-block; margin-right: 6px; padding: 0 5px;
    border-radius: 3px; border: 1px solid var(--text-faint); color: var(--text-muted);
    font-size: 10px; font-weight: 600; line-height: 15px; vertical-align: 1px;
  }
  .rate { display: flex; flex: none; margin-left: 4px; }

  .center { display: flex; flex-direction: column; align-items: stretch; gap: 0; }
  .controls { display: flex; align-items: center; justify-content: center; gap: 16px; margin-bottom: 4px; }
  .icon-btn.big { width: 32px; height: 32px; color: var(--text-muted); }
  .icon-btn.big:hover { color: var(--text); }
  .play {
    width: 36px; height: 36px; border-radius: 50%;
    display: grid; place-items: center;
    background: var(--text); color: #000;
    transition: transform var(--dur-fast) var(--ease), background var(--dur-fast) var(--ease);
  }
  .play:hover { transform: scale(1.06); }
  .play:active { transform: scale(0.94); background: #dcdcdc; }
  .play:disabled { opacity: 0.35; transform: none; }
  .seek { display: flex; align-items: center; gap: 8px; }
  .time { font-size: 12px; color: var(--text-muted); font-variant-numeric: tabular-nums; min-width: 36px; }
  .time.end { text-align: right; }

  /* Toggles read as on through a dot under the icon, not by recoloring it. */
  .bar .icon-btn.active { position: relative; color: var(--accent); }
  .bar .icon-btn.active:hover { color: var(--accent-hover); }
  .bar .icon-btn.active::after {
    content: "";
    position: absolute; left: 50%; bottom: 1px;
    width: 4px; height: 4px; margin-left: -2px; border-radius: 50%;
    background: var(--accent);
  }
  .rate .icon-btn.active::after { display: none; }

  .extras { display: flex; align-items: center; justify-content: flex-end; gap: 2px; }
  .sleep { position: relative; display: flex; align-items: center; }
  .countdown { font-size: 11px; font-weight: 600; color: var(--text-muted); font-variant-numeric: tabular-nums; margin-right: 4px; }
  .menu {
    position: absolute; bottom: calc(100% + 12px); right: -40px;
    width: 196px; padding: 4px;
    border-radius: var(--radius-sm);
    background: var(--bg-highlight);
    box-shadow: var(--shadow-pop);
    transform-origin: bottom right;
    z-index: 5;
  }
  .menu-title { font-size: 12px; color: var(--text-faint); padding: 6px 10px 4px; }
  .menu button { display: block; width: 100%; text-align: left; padding: 10px 12px; border-radius: 2px; font-size: 14px; transition: background var(--dur-fast) var(--ease); }
  .menu button:hover { background: rgba(255, 255, 255, 0.1); }
  .menu .off { color: var(--danger); }

  .volume { display: flex; align-items: center; gap: 2px; margin: 0 6px 0 4px; }
  .volume input {
    -webkit-appearance: none;
    appearance: none;
    width: 96px; height: 4px; border-radius: 2px;
    background: linear-gradient(90deg, var(--fill-color, var(--text)) 0 var(--fill), rgba(255, 255, 255, 0.3) var(--fill) 100%);
    cursor: pointer;
  }
  .volume:hover input { --fill-color: var(--accent); }
  .volume input::-webkit-slider-thumb {
    -webkit-appearance: none;
    width: 12px; height: 12px; border-radius: 50%;
    background: #fff;
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.5);
    opacity: 0;
    transition: opacity var(--dur-fast) var(--ease);
  }
  .volume:hover input::-webkit-slider-thumb, .volume input:focus-visible::-webkit-slider-thumb { opacity: 1; }

  @media (max-width: 900px) {
    .bar { grid-template-columns: minmax(160px, 1fr) minmax(260px, 2fr) auto; gap: 12px; }
    .rate, .volume input { display: none; }
  }
</style>
