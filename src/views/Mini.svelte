<script lang="ts">
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { onMount } from "svelte";
  import Icon from "../components/Icon.svelte";
  import WaveSlider from "../components/WaveSlider.svelte";
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
  const disliked = $derived(video?.likeStatus === 0);
  const muted = $derived(player?.player.muted ?? false);
  const volume = $derived(Math.round(player?.player.volume ?? 0));

  function apply(s: PlayerState) {
    player = s;
    clock.sync(s.player.videoProgress);
  }

  onMount(() => {
    api.getPlayerState().then(apply);
    const unlisten = onPlayerState(apply);
    const stop = clock.start();
    return () => {
      unlisten.then((f) => f());
      stop();
    };
  });

  function seek(seconds: number) {
    clock.sync(seconds);
    api.command("seekTo", seconds);
  }

  function wheelVolume(e: WheelEvent) {
    e.preventDefault();
    const next = Math.max(0, Math.min(100, volume + (e.deltaY < 0 ? 5 : -5)));
    api.command("setVolume", next);
  }

  async function togglePin() {
    pinned = !pinned;
    await appWindow.setAlwaysOnTop(pinned);
  }
</script>

<div class="mini" data-tauri-drag-region>
  <div class="art" onwheel={wheelVolume} title="Scroll to change volume ({volume}%)">
    {#if art}<img src={art} alt="" />{:else}<Icon name="music" size={36} />{/if}
  </div>

  <div class="content" data-tauri-drag-region>
    <div class="top" data-tauri-drag-region>
      <div class="meta" data-tauri-drag-region>
        <div class="title" title={video?.title}>{video?.title ?? "Nothing playing"}</div>
        <div class="artist">{video ? video.author : "Start something in YouTube Music"}</div>
      </div>
      <div class="window">
        <button class="icon-btn small" class:active={pinned} title={pinned ? "Unpin" : "Keep on top"} onclick={togglePin}><Icon name="pin" size={15} /></button>
        <button class="icon-btn small" title="Open YTMusic" onclick={() => api.showMainWindow()}><Icon name="expand" size={15} /></button>
        <button class="icon-btn small" title="Close" onclick={() => appWindow.close()}><Icon name="close" size={16} /></button>
      </div>
    </div>

    <div class="progress">
      <span class="time">{formatTime(elapsed)}</span>
      <WaveSlider value={elapsed} max={duration} playing={playing} disabled={!video || video.isLive} height={26} onseek={seek} />
      <span class="time">{video?.isLive ? "LIVE" : formatTime(duration)}</span>
    </div>

    <div class="controls">
      <button class="icon-btn" class:active={disliked} title="Dislike" disabled={!video} onclick={() => api.command("toggleDislike")}><Icon name="dislike" size={18} /></button>
      <button class="icon-btn" title="Previous" disabled={!video} onclick={() => api.command("previous")}><Icon name="previous" size={22} /></button>
      <button class="play" title={playing ? "Pause" : "Play"} disabled={!video} onclick={() => api.command("playPause")}>
        <Icon name={playing ? "pause" : "play"} size={24} />
      </button>
      <button class="icon-btn" title="Next" disabled={!video} onclick={() => api.command("next")}><Icon name="next" size={22} /></button>
      <button class="icon-btn" class:active={liked} title="Like" disabled={!video} onclick={() => api.command("toggleLike")}><Icon name="like" size={18} /></button>
      <button class="icon-btn" title={muted ? "Unmute" : "Mute"} disabled={!video} onclick={() => api.command(muted ? "unmute" : "mute")}>
        <Icon name={muted ? "muted" : "volume"} size={18} />
      </button>
      <button class="icon-btn" title="Close track" disabled={!video} onclick={() => api.command("closeTrack")}><Icon name="stop" size={16} /></button>
    </div>
  </div>
</div>

<style>
  .mini {
    position: relative;
    height: 100%;
    display: flex;
    gap: 14px;
    padding: 14px;
    overflow: hidden;
    background: var(--bg-panel);
    border: 1px solid var(--border);
  }

  .art {
    width: 126px; height: 126px; flex: none;
    border-radius: var(--radius-sm); overflow: hidden;
    display: grid; place-items: center;
    background: var(--bg-highlight);
    box-shadow: var(--shadow-card);
  }
  .art img { width: 100%; height: 100%; object-fit: cover; }

  .content { flex: 1; min-width: 0; display: flex; flex-direction: column; justify-content: space-between; }
  .top { display: flex; gap: 4px; align-items: flex-start; }
  .meta { flex: 1; min-width: 0; padding-top: 2px; }
  .title { font-weight: 700; font-size: 15px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .artist { color: var(--text-muted); font-size: 12.5px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .window { display: flex; margin: -6px -6px 0 0; opacity: 0.6; transition: opacity 0.2s; }
  .mini:hover .window { opacity: 1; }
  .icon-btn.small { width: 26px; height: 26px; }

  .progress { display: flex; align-items: center; gap: 8px; }
  .time { font-size: 11px; color: var(--text-muted); font-variant-numeric: tabular-nums; min-width: 32px; }
  .time:last-child { text-align: right; }

  .controls { display: flex; align-items: center; justify-content: space-between; }
  .play {
    width: 42px; height: 42px; border-radius: 50%;
    display: grid; place-items: center;
    background: #fff; color: #000;
    transition: transform var(--dur-fast) var(--ease), background var(--dur-fast) var(--ease);
  }
  .play:hover { transform: scale(1.06); }
  .play:active { transform: scale(0.94); background: #dcdcdc; }
  .play:disabled { opacity: 0.4; }
</style>
