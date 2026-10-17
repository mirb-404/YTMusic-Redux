<script lang="ts">
  import { onMount } from "svelte";
  import Icon from "./Icon.svelte";
  import WaveSlider from "./WaveSlider.svelte";
  import { api, bestThumbnail, formatTime, type Memory, type PlayerState } from "../lib/api";
  import type { ProgressClock } from "../lib/progress.svelte";

  let {
    player,
    memory,
    clock,
    showVideos,
    fullscreen
  }: { player: PlayerState | null; memory: Memory | null; clock: ProgressClock; showVideos: boolean; fullscreen: boolean } = $props();

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

<footer class="bar">
  <section class="track">
    <button class="art" title="Now playing" disabled={!video} onclick={() => api.command("togglePlayerPage")}>
      {#if art}<img src={art} alt="" />{:else}<Icon name="music" size={26} />{/if}
      <span class="overlay"><Icon name="expand" size={18} /></span>
    </button>
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
    <div class="controls">
      <button class="icon-btn" title="Shuffle" disabled={!video} onclick={() => api.command("shuffle")}><Icon name="shuffle" size={19} /></button>
      <button class="icon-btn big" title="Previous" disabled={!video} onclick={() => api.command("previous")}><Icon name="previous" size={26} /></button>
      <button class="play" title={playing ? "Pause" : "Play"} disabled={!video} onclick={() => api.command("playPause")}>
        <Icon name={playing ? "pause" : "play"} size={26} />
      </button>
      <button class="icon-btn big" title="Next" disabled={!video} onclick={() => api.command("next")}><Icon name="next" size={26} /></button>
      <button
        class="icon-btn"
        class:active={repeatMode > 0}
        title={["Repeat off", "Repeat all", "Repeat one"][repeatMode] ?? "Repeat"}
        disabled={!video}
        onclick={() => api.command("repeatMode", (repeatMode + 1) % 3)}
      >
        <Icon name={repeatMode === 2 ? "repeatOne" : "repeat"} size={19} />
      </button>
    </div>
    <div class="seek">
      <span class="time">{formatTime(elapsed)}</span>
      <WaveSlider value={elapsed} max={duration} playing={playing && !buffering} disabled={!video || video.isLive || adPlaying} onseek={seek} />
      <span class="time end">{video?.isLive ? "LIVE" : formatTime(duration)}</span>
    </div>
  </section>

  <section class="extras">
    <button class="icon-btn" title="Add to playlist" disabled={!video} onclick={() => api.command("addToPlaylist")}><Icon name="playlistAdd" size={20} /></button>
    <div class="sleep">
      <button class="icon-btn" class:active={sleepLeft !== null} title="Sleep timer" onclick={() => (sleepOpen = !sleepOpen)}>
        <Icon name="timer" size={19} />
      </button>
      {#if sleepLeft !== null}<span class="countdown">{formatTime(sleepLeft)}</span>{/if}
      {#if sleepOpen}
        <div class="menu" role="menu">
          <div class="menu-title">Pause music after</div>
          {#each sleepOptions as m (m)}
            <button role="menuitem" onclick={() => setSleep(m)}>{m < 60 ? `${m} minutes` : `${m / 60} hour${m > 60 ? "s" : ""}`}</button>
          {/each}
          {#if sleepLeft !== null}<button role="menuitem" class="off" onclick={() => setSleep(0)}>Turn off timer</button>{/if}
        </div>
      {/if}
    </div>
    <button class="icon-btn" class:active={nowPlayingOpen} title="Lyrics" onclick={() => api.setNowPlayingOpen(!nowPlayingOpen)}><Icon name="lyrics" size={20} /></button>
    <button
      class="icon-btn"
      class:active={showVideos}
      title={showVideos ? "Music videos on (switch to audio only)" : "Audio only (switch to music videos)"}
      onclick={() => api.setSetting("playback.showVideos", !showVideos)}
    >
      <Icon name={showVideos ? "video" : "videoOff"} size={19} />
    </button>
    <button class="icon-btn" title="Queue" disabled={!video} onclick={() => api.command("togglePlayerPage")}><Icon name="queue" size={20} /></button>
    <div class="volume" onwheel={wheelVolume}>
      <button class="icon-btn" title={muted ? "Unmute" : "Mute"} onclick={() => api.command(muted ? "unmute" : "mute")}>
        <Icon name={muted || volume === 0 ? "muted" : volume < 50 ? "volumeLow" : "volume"} size={19} />
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
    <button class="icon-btn" title={fullscreen ? "Exit full screen (F11)" : "Full screen (F11)"} onclick={() => api.toggleFullscreen()}>
      <Icon name={fullscreen ? "fullscreenExit" : "fullscreen"} size={20} />
    </button>
  </section>
</footer>

<style>
  .bar {
    height: 88px;
    flex: none;
    display: grid;
    grid-template-columns: minmax(240px, 1fr) minmax(320px, 520px) minmax(240px, 1fr);
    align-items: center;
    gap: 20px;
    padding: 0 18px 0 14px;
    background: #0f0f14;
    border-top: 1px solid var(--border);
  }
  section { position: relative; min-width: 0; }

  .track { display: flex; align-items: center; gap: 12px; }
  .art {
    position: relative;
    width: 56px; height: 56px; flex: none;
    border-radius: 8px; overflow: hidden;
    display: grid; place-items: center;
    background: var(--surface-hover);
    color: var(--text-faint);
  }
  .art img { width: 100%; height: 100%; object-fit: cover; }
  .overlay {
    position: absolute; inset: 0;
    display: grid; place-items: center;
    background: rgba(0, 0, 0, 0.45); color: #fff;
    opacity: 0; transition: opacity 0.2s;
  }
  .art:not(:disabled):hover .overlay { opacity: 1; }
  .meta { min-width: 0; flex: 0 1 auto; }
  .title { font-weight: 700; font-size: 14.5px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .sub { color: var(--text-muted); font-size: 12.5px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; margin-top: 1px; }
  .dot { margin: 0 6px; color: var(--text-faint); }
  .ad {
    display: inline-block; margin-right: 6px; padding: 0 6px;
    border-radius: 4px; background: var(--warning); color: #111;
    font-size: 10.5px; font-weight: 800; vertical-align: 2px;
  }
  .rate { display: flex; flex: none; }

  .center { display: flex; flex-direction: column; align-items: stretch; gap: 2px; }
  .controls { display: flex; align-items: center; justify-content: center; gap: 10px; }
  .icon-btn.big { width: 38px; height: 38px; color: var(--text); }
  .play {
    width: 42px; height: 42px; border-radius: 50%;
    display: grid; place-items: center;
    background: #fff; color: #0b0b0f;
    transition: transform 0.12s var(--ease), background 0.12s;
  }
  .play:hover { background: #e9e9ee; }
  .play:active { transform: scale(0.95); }
  .play:disabled { opacity: 0.4; }
  .seek { display: flex; align-items: center; gap: 6px; }
  .time { font-size: 11.5px; color: var(--text-muted); font-variant-numeric: tabular-nums; min-width: 38px; }
  .time.end { text-align: right; }

  .extras { display: flex; align-items: center; justify-content: flex-end; gap: 2px; }
  .sleep { position: relative; display: flex; align-items: center; }
  .countdown { font-size: 11px; font-weight: 700; color: var(--accent); font-variant-numeric: tabular-nums; margin-right: 4px; }
  .menu {
    position: absolute; bottom: calc(100% + 10px); right: -40px;
    width: 190px; padding: 6px;
    border-radius: 14px;
    background: rgba(24, 24, 32, 0.98);
    border: 1px solid var(--border);
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.4);
    z-index: 5;
  }
  .menu-title { font-size: 11px; text-transform: uppercase; letter-spacing: 0.7px; color: var(--text-faint); padding: 6px 10px; }
  .menu button { display: block; width: 100%; text-align: left; padding: 8px 10px; border-radius: 8px; }
  .menu button:hover { background: var(--surface-hover); }
  .menu .off { color: var(--danger); }

  .volume { display: flex; align-items: center; gap: 2px; margin: 0 6px 0 4px; }
  .volume input {
    -webkit-appearance: none;
    appearance: none;
    width: 84px; height: 4px; border-radius: 2px;
    background: linear-gradient(90deg, var(--accent) 0 var(--fill), rgba(255, 255, 255, 0.16) var(--fill) 100%);
    cursor: pointer;
  }
  .volume input::-webkit-slider-thumb {
    -webkit-appearance: none;
    width: 12px; height: 12px; border-radius: 50%;
    background: #fff;
  }

  @media (max-width: 900px) {
    .bar { grid-template-columns: minmax(160px, 1fr) minmax(260px, 2fr) auto; gap: 12px; }
    .rate, .volume input { display: none; }
  }

</style>
