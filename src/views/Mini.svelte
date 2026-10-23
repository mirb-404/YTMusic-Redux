<script lang="ts">
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { onMount } from "svelte";
  import { fade } from "svelte/transition";
  import Icon from "../components/Icon.svelte";
  import { ProgressClock } from "../lib/progress.svelte";
  import { api, bestThumbnail, formatTime, onPlayerState, type PlayerState } from "../lib/api";

  const appWindow = getCurrentWindow();
  let player = $state<PlayerState | null>(null);
  let pinned = $state(true);
  const clock = new ProgressClock();

  const video = $derived(player?.video ?? null);
  const playing = $derived(player?.player.trackState === 1);
  const duration = $derived(video?.durationSeconds ?? 0);
  const art = $derived(bestThumbnail(video?.thumbnails, 200));
  const elapsed = $derived(clock.elapsed(playing, duration));
  const liked = $derived(video?.likeStatus === 2);
  const muted = $derived(player?.player.muted ?? false);
  const seekable = $derived(!!video && !video.isLive && duration > 0);
  let volumeDraft = $state<number | null>(null);
  const volume = $derived(volumeDraft ?? Math.round(player?.player.volume ?? 0));
  // While the seek bar is dragged it shows the drag position, not playback.
  let seekDraft = $state<number | null>(null);
  const position = $derived(seekDraft ?? elapsed);

  function apply(s: PlayerState) {
    player = s;
    clock.sync(s.player.videoProgress);
  }

  onMount(() => {
    api.getPlayerState().then(apply);
    const unlisten = onPlayerState(apply);
    return () => unlisten.then((f) => f());
  });

  // The clock only needs frames while time advances.
  $effect(() => {
    if (playing) return clock.start();
  });

  function seek(seconds: number) {
    seekDraft = null;
    clock.sync(seconds);
    api.command("seekTo", seconds);
  }

  // Dragging sends at most one command per 60 ms; the slider shows the draft meanwhile.
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

  async function togglePin() {
    pinned = !pinned;
    await appWindow.setAlwaysOnTop(pinned);
  }
</script>

<div class="mini" data-tauri-drag-region onwheel={wheelVolume}>
  <!-- The artwork, blurred and dimmed, colors the whole window. -->
  {#if art}{#key art}<img class="backdrop" src={art} alt="" aria-hidden="true" transition:fade={{ duration: 500 }} />{/key}{/if}

  <div class="art" data-tauri-drag-region>
    {#if art}{#key art}<img src={art} alt="" in:fade={{ duration: 250 }} />{/key}{:else}<Icon name="music" size={36} />{/if}
  </div>

  <div class="content" data-tauri-drag-region>
    <div class="meta" data-tauri-drag-region>
      <div class="title" title={video?.title} data-tauri-drag-region>{video?.title ?? "Nothing playing"}</div>
      <div class="artist" data-tauri-drag-region>{video ? video.author : "Play something in YTMusic"}</div>
    </div>

    <div class="seek">
      <input
        type="range"
        class="bar"
        min="0"
        max={Math.max(1, duration)}
        step="any"
        value={position}
        disabled={!seekable}
        aria-label="Seek"
        style:--fill="{seekable ? (position / duration) * 100 : 0}%"
        oninput={(e) => (seekDraft = Number(e.currentTarget.value))}
        onchange={(e) => seek(Number(e.currentTarget.value))}
      />
      <div class="times">
        <span>{formatTime(position)}</span>
        <span>{video?.isLive ? "LIVE" : formatTime(duration)}</span>
      </div>
    </div>

    <div class="controls">
      <button class="ctl" class:on={liked} title={liked ? "Remove like" : "Like"} disabled={!video} onclick={() => api.command("toggleLike")}>
        <Icon name={liked ? "like" : "likeOutline"} size={18} />
      </button>
      <span class="transport">
        <button class="ctl" title="Previous" disabled={!video} onclick={() => api.command("previous")}><Icon name="previous" size={24} /></button>
        <button class="play" title={playing ? "Pause" : "Play"} disabled={!video} onclick={() => api.command("playPause")}>
          <Icon name={playing ? "pause" : "play"} size={22} />
        </button>
        <button class="ctl" title="Next" disabled={!video} onclick={() => api.command("next")}><Icon name="next" size={24} /></button>
      </span>
      <div class="volume">
        <button class="ctl" title={muted ? "Unmute" : "Mute"} onclick={() => api.command(muted ? "unmute" : "mute")}>
          <Icon name={muted || volume === 0 ? "muted" : volume < 50 ? "volumeLow" : "volume"} size={18} />
        </button>
        <!-- Opens above the button on hover: the controls row has no room for a slider. -->
        <div class="volume-pop">
          <input
            type="range"
            class="bar"
            min="0"
            max="100"
            value={muted ? 0 : volume}
            aria-label="Volume"
            style:--fill="{muted ? 0 : volume}%"
            oninput={(e) => setVolume(Number(e.currentTarget.value))}
          />
          <span class="volume-value">{muted ? 0 : volume}</span>
        </div>
      </div>
    </div>
  </div>

  <div class="window">
    <button class:on={pinned} title={pinned ? "Stop keeping on top" : "Keep on top"} onclick={togglePin}><Icon name="pin" size={14} /></button>
    <button title="Open YTMusic" onclick={() => api.showMainWindow()}><Icon name="expand" size={14} /></button>
    <button title="Close" onclick={() => appWindow.close()}><Icon name="close" size={15} /></button>
  </div>
</div>

<style>
  .mini {
    position: relative; isolation: isolate; overflow: hidden;
    height: 100%;
    display: flex; gap: 14px;
    padding: 12px;
    background: var(--bg-panel);
    border: 1px solid var(--border);
  }
  .backdrop {
    position: absolute; inset: -40px; z-index: -1;
    width: calc(100% + 80px); height: calc(100% + 80px);
    object-fit: cover;
    filter: blur(36px) saturate(1.5) brightness(0.45);
    pointer-events: none;
  }

  .art {
    width: 130px; height: 130px; flex: none;
    border-radius: 6px; overflow: hidden;
    display: grid; place-items: center;
    background: var(--bg-highlight); color: var(--text-faint);
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.5);
  }
  .art img { width: 100%; height: 100%; object-fit: cover; }

  .content { flex: 1; min-width: 0; display: flex; flex-direction: column; justify-content: space-between; padding: 2px 0 0; }
  /* Leaves room for the window buttons in the top right corner. */
  .meta { min-width: 0; padding-right: 64px; }
  .title { font-size: 14px; font-weight: 700; line-height: 1.3; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .artist { margin-top: 1px; font-size: 12.5px; color: rgba(255, 255, 255, 0.7); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }

  /* One slider style for seek and volume: a thin track that thickens and shows its knob on hover. */
  .bar {
    -webkit-appearance: none; appearance: none;
    display: block; width: 100%; height: 12px; margin: 0;
    background: transparent; cursor: pointer;
  }
  .bar::-webkit-slider-runnable-track {
    height: 4px; border-radius: 2px;
    background: linear-gradient(90deg, var(--fill-color, #fff) 0 var(--fill), rgba(255, 255, 255, 0.25) var(--fill) 100%);
  }
  .bar::-webkit-slider-thumb {
    -webkit-appearance: none;
    width: 12px; height: 12px; margin-top: -4px; border-radius: 50%;
    background: #fff; box-shadow: 0 1px 4px rgba(0, 0, 0, 0.5);
    transform: scale(0); transition: transform var(--dur-fast) var(--ease);
  }
  .bar:hover, .bar:focus-visible { --fill-color: var(--accent); }
  .bar:hover::-webkit-slider-thumb, .bar:focus-visible::-webkit-slider-thumb, .bar:active::-webkit-slider-thumb { transform: scale(1); }
  .bar:disabled { cursor: default; opacity: 0.5; }
  .bar:disabled::-webkit-slider-thumb { transform: scale(0); }
  .bar:focus-visible { outline: none; }

  .times { display: flex; justify-content: space-between; margin-top: 1px; font-size: 10.5px; color: rgba(255, 255, 255, 0.6); font-variant-numeric: tabular-nums; }

  .controls { display: flex; align-items: center; justify-content: space-between; margin: 0 -6px; }
  .transport { display: flex; align-items: center; gap: 6px; }
  .ctl {
    width: 32px; height: 32px; border-radius: 50%;
    display: grid; place-items: center;
    color: rgba(255, 255, 255, 0.75);
    transition: color var(--dur-fast) var(--ease), transform var(--dur-fast) var(--ease);
  }
  .ctl:hover { color: #fff; transform: scale(1.06); }
  .ctl:active { transform: scale(0.94); }
  .ctl.on { color: var(--accent); }
  .ctl:disabled { opacity: 0.35; pointer-events: none; }
  .play {
    width: 38px; height: 38px; border-radius: 50%;
    display: grid; place-items: center;
    background: #fff; color: #000;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.35);
    transition: transform var(--dur-fast) var(--ease), background var(--dur-fast) var(--ease);
  }
  .play:hover { transform: scale(1.06); }
  .play:active { transform: scale(0.94); background: #dcdcdc; }
  .play:disabled { opacity: 0.4; pointer-events: none; }

  .volume { position: relative; display: flex; }
  .volume-pop {
    position: absolute; right: 0; bottom: calc(100% + 4px); z-index: 2;
    display: flex; align-items: center; gap: 8px;
    width: 150px; height: 30px; padding: 0 10px 0 12px; border-radius: 6px;
    background: rgba(40, 40, 40, 0.96); box-shadow: var(--shadow-pop);
    opacity: 0; transform: translateY(4px); pointer-events: none;
    transition: opacity var(--dur) var(--ease), transform var(--dur) var(--ease);
  }
  /* Bridges the gap to the button, so moving the pointer up keeps it open. */
  .volume-pop::after { content: ""; position: absolute; left: 0; right: 0; top: 100%; height: 6px; }
  .volume:hover .volume-pop, .volume:focus-within .volume-pop { opacity: 1; transform: none; pointer-events: auto; }
  .volume-pop .bar { flex: 1; min-width: 0; }
  .volume-value { min-width: 20px; text-align: right; font-size: 11px; font-variant-numeric: tabular-nums; color: var(--text-muted); }

  .window {
    position: absolute; top: 6px; right: 6px;
    display: flex; gap: 2px;
    opacity: 0; transition: opacity var(--dur) var(--ease);
  }
  .mini:hover .window, .window:focus-within { opacity: 1; }
  .window button {
    width: 24px; height: 24px; border-radius: 4px;
    display: grid; place-items: center;
    color: rgba(255, 255, 255, 0.7);
    transition: background var(--dur-fast) var(--ease), color var(--dur-fast) var(--ease);
  }
  .window button:hover { background: rgba(255, 255, 255, 0.12); color: #fff; }
  .window button.on { color: var(--accent); }
  .window button:last-child:hover { background: #c42b1c; color: #fff; }
</style>
