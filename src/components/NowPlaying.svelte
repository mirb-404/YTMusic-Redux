<script lang="ts">
  import type { Snippet } from "svelte";
  import { fly } from "svelte/transition";
  import { cubicOut } from "svelte/easing";
  import Icon from "./Icon.svelte";
  import { api, bestThumbnail, type PlayerState } from "../lib/api";
  import type { LyricsTrack } from "../lib/lyrics.svelte";

  let {
    player,
    lyrics,
    lineIndex,
    elapsed,
    showVideos,
    controls,
    drag,
    onclose,
    onseek
  }: {
    player: PlayerState | null;
    lyrics: LyricsTrack;
    lineIndex: number;
    elapsed: number;
    showVideos: boolean;
    /** Window controls, when this view is the window's only header. */
    controls?: Snippet;
    drag?: boolean;
    onclose: () => void;
    onseek: (seconds: number) => void;
  } = $props();

  const video = $derived(player?.video ?? null);
  const art = $derived(bestThumbnail(video?.thumbnails, 544));
  const synced = $derived(lyrics.data?.synced ?? null);

  // The next track is announced during the last seconds of the current one.
  const UP_NEXT_SECONDS = 15;
  const upNext = $derived.by(() => {
    const q = player?.player.queue;
    if (!q || q.repeatMode === 2 || q.selectedItemIndex < 0) return null;
    return q.items[q.selectedItemIndex + 1] ?? q.automixItems[0] ?? (q.repeatMode === 1 ? q.items[0] : null) ?? null;
  });
  const left = $derived(video ? video.durationSeconds - elapsed : Infinity);
  let dismissed = $state("");
  const showUpNext = $derived(
    !!upNext && !!video && !video.isLive && video.durationSeconds > UP_NEXT_SECONDS * 3 && left > 0.5 && left <= UP_NEXT_SECONDS && dismissed !== video.id
  );

  // Each track gets its own vibrant palette, picked from its id, for the lyric colors and the
  // light drifting behind them.
  const PALETTES = [
    ["#ff5f6d", "#ffc371", "#ff8fab"],
    ["#7f7fd5", "#86a8e7", "#91eae4"],
    ["#f857a6", "#ff5858", "#ffb199"],
    ["#43e97b", "#38f9d7", "#a8ff78"],
    ["#fa709a", "#fee140", "#ff9a8b"],
    ["#a18cd1", "#fbc2eb", "#8fd3f4"],
    ["#f093fb", "#f5576c", "#ffd166"]
  ];
  const palette = $derived.by(() => {
    let hash = 0;
    for (const ch of video?.id ?? "") hash = (hash * 31 + ch.charCodeAt(0)) | 0;
    return PALETTES[Math.abs(hash) % PALETTES.length];
  });

  let list = $state<HTMLElement>();
  // Manual scrolling pauses auto-scroll for a few seconds.
  let userScrollUntil = 0;

  $effect(() => {
    const i = lineIndex;
    if (!list || !synced || Date.now() < userScrollUntil) return;
    const line = list.querySelector<HTMLElement>(`[data-line="${Math.max(0, i)}"]`);
    line?.scrollIntoView({ block: "center", behavior: "smooth" });
  });
</script>

<main class="now-playing" style:--c1={palette[0]} style:--c2={palette[1]} style:--c3={palette[2]}>
  <div class="glow" aria-hidden="true"><span></span><span></span><span></span></div>
  <header class:with-controls={!!controls} data-tauri-drag-region={controls ? drag : undefined}>
    <button class="icon-btn" title="Close (Esc)" onclick={onclose}><Icon name="chevronDown" size={22} /></button>
    <span class="label" data-tauri-drag-region={controls ? drag : undefined}>Now playing</span>
    <span class="push" data-tauri-drag-region={controls ? drag : undefined}></span>
    <button class="chip" class:on={showVideos} title={showVideos ? "Playing music videos" : "Playing audio only"} onclick={() => api.setSetting("playback.showVideos", !showVideos)}>
      <Icon name={showVideos ? "video" : "videoOff"} size={16} />{showVideos ? "Video" : "Audio only"}
    </button>
    <button class="chip" title="Equalizer and playback" onclick={() => api.openSettings("audio")}><Icon name="equalizer" size={16} />Audio</button>
    {#if controls}<div class="controls">{@render controls()}</div>{/if}
  </header>

  <div class="content">
    <section class="track">
      <div class="art">
        {#if art}<img src={art} alt="" />{:else}<Icon name="music" size={64} />{/if}
      </div>
      <h1 title={video?.title}>{video?.title ?? "Nothing playing"}</h1>
      <p>{video ? [video.author, video.album].filter(Boolean).join(" · ") : "Pick something to play"}</p>
    </section>

    <section class="lyrics" class:synced={!!synced && !lyrics.loading} bind:this={list} onwheel={() => (userScrollUntil = Date.now() + 4000)}>
      {#if !video}
        <p class="state">Play a song to see its lyrics.</p>
      {:else if lyrics.loading}
        <p class="state">Loading lyrics...</p>
      {:else if lyrics.error}
        <p class="state">{lyrics.error}</p>
        <button class="btn" onclick={() => lyrics.load(video.id, true)}><Icon name="refresh" size={16} />Retry</button>
      {:else if synced}
        {#each synced as line, i (i)}
          <button
            class="line"
            class:active={i === lineIndex}
            class:past={i < lineIndex}
            style:--depth={Math.min(4, Math.abs(i - lineIndex))}
            data-line={i}
            onclick={() => onseek(line.time)}
          >
            {#if line.text}{line.text}{:else}<span class="beat"><i></i><i></i><i></i></span>{/if}
          </button>
        {/each}
      {:else if lyrics.data?.plain}
        <p class="plain">{lyrics.data.plain}</p>
      {:else if lyrics.data?.instrumental}
        <p class="state">Instrumental</p>
      {:else}
        <p class="state">No lyrics found for this track.</p>
      {/if}
      {#if lyrics.data}<p class="credit">Lyrics from LRCLIB</p>{/if}
    </section>
  </div>

  {#if showUpNext && upNext}
    <aside class="up-next" in:fly={{ y: 24, duration: 380, easing: cubicOut }} out:fly={{ y: 16, duration: 200 }}>
      {#if bestThumbnail(upNext.thumbnails, 120)}<img src={bestThumbnail(upNext.thumbnails, 120)} alt="" />{/if}
      <div class="next-text">
        <span class="next-label">Up next · {Math.ceil(left)}s</span>
        <span class="next-title">{upNext.title}</span>
        <span class="next-artist">{upNext.author}</span>
      </div>
      <button class="icon-btn play" title="Play now" onclick={() => api.command("next")}><Icon name="next" size={18} /></button>
      <button class="icon-btn" title="Dismiss" onclick={() => (dismissed = video?.id ?? "")}><Icon name="close" size={16} /></button>
      <span class="countdown" style:transform="scaleX({left / UP_NEXT_SECONDS})"></span>
    </aside>
  {/if}
</main>

<style>
  .now-playing { position: relative; isolation: isolate; overflow: hidden; flex: 1; min-height: 0; display: flex; flex-direction: column; background: var(--bg); }

  /* Soft light in the track's colors drifting behind everything. Radial gradients rather than
     a blur filter: a large blur keeps big offscreen buffers in GPU memory and re-renders them
     every frame, while gradients only move. */
  .glow { position: absolute; inset: 0; z-index: -1; pointer-events: none; }
  .glow span {
    position: absolute; width: 60vmax; height: 60vmax;
    background: radial-gradient(circle, color-mix(in srgb, var(--color) 28%, transparent) 0%, transparent 65%);
    animation: drift 22s ease-in-out infinite alternate;
  }
  .glow span:nth-child(1) { --color: var(--c1); left: -18vmax; top: -22vmax; }
  .glow span:nth-child(2) { --color: var(--c2); right: -22vmax; top: 8vmax; animation-duration: 28s; animation-delay: -8s; }
  .glow span:nth-child(3) { --color: var(--c3); left: 18vmax; bottom: -32vmax; animation-duration: 34s; animation-delay: -15s; }
  @keyframes drift {
    from { transform: translate(0, 0) scale(1); }
    to { transform: translate(6vmax, -4vmax) scale(1.15); }
  }
  header { display: flex; align-items: center; gap: 8px; padding: 12px 20px 0 14px; }
  /* The window controls sit flush with the window's top right corner. */
  header.with-controls { padding-right: 0; }
  .controls { align-self: flex-start; display: flex; align-items: center; gap: 2px; margin: -12px 0 0 8px; }
  .label { font-size: 12px; font-weight: 700; letter-spacing: 0.6px; text-transform: uppercase; color: var(--text-muted); }
  .push { flex: 1; }
  .chip {
    display: inline-flex; align-items: center; gap: 6px;
    height: 30px; padding: 0 12px; border-radius: 15px;
    border: 1px solid var(--border); color: var(--text-muted);
    font-size: 12.5px; font-weight: 600;
    transition: background 0.12s, color 0.12s;
  }
  .chip:hover { background: var(--surface-hover); color: var(--text); }
  .chip.on { background: var(--text); border-color: transparent; color: var(--bg); }

  .content {
    flex: 1; min-height: 0;
    display: grid; grid-template-columns: minmax(240px, 400px) minmax(0, 1fr);
    gap: 56px; padding: 24px 48px 0;
  }
  .track { align-self: center; min-width: 0; padding-bottom: 32px; }
  .art {
    width: 100%; aspect-ratio: 1; border-radius: 12px; overflow: hidden;
    display: grid; place-items: center;
    background: var(--surface-hover); color: var(--text-faint);
  }
  .art img { width: 100%; height: 100%; object-fit: cover; }
  h1 { margin: 20px 0 4px; font-size: 24px; line-height: 1.25; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .track p { margin: 0; color: var(--text-muted); font-size: 14.5px; }

  .lyrics { overflow-y: auto; padding: 24px 8px 32px 0; scrollbar-width: none; }
  .lyrics.synced { padding: 30vh 8px 30vh 0; }
  .lyrics::-webkit-scrollbar { display: none; }
  .line {
    display: block; width: 100%; text-align: left;
    padding: 8px 0; font-size: 30px; font-weight: 800; line-height: 1.25; letter-spacing: -0.01em;
    color: rgba(255, 255, 255, calc(0.42 - var(--depth) * 0.06));
    filter: blur(calc(var(--depth) * 0.45px));
    transform-origin: left center;
    transform: scale(0.96);
    transition: color 0.5s var(--ease), filter 0.5s var(--ease), transform 0.6s cubic-bezier(0.34, 1.56, 0.64, 1);
  }
  .line.past { color: rgba(255, 255, 255, calc(0.3 - var(--depth) * 0.04)); }
  /* The current line: the track's colors flowing through the text, a springy pop and a glow. */
  .line.active {
    transform: scale(1.04);
    filter: drop-shadow(0 0 22px color-mix(in srgb, var(--c1) 45%, transparent));
    background: linear-gradient(100deg, var(--c1), var(--c2), var(--c3), var(--c1));
    background-size: 300% 100%;
    -webkit-background-clip: text;
    background-clip: text;
    color: transparent;
    animation: flow 6s linear infinite;
  }
  .line:not(.active):hover { color: rgba(255, 255, 255, 0.85); filter: none; }
  @keyframes flow { to { background-position: -300% 0; } }

  /* Instrumental breaks: three dots bouncing in the track's colors. */
  .beat { display: inline-flex; gap: 8px; padding: 10px 0; }
  .beat i { width: 10px; height: 10px; border-radius: 50%; background: var(--c1); opacity: 0.5; animation: bounce 1.2s ease-in-out infinite; }
  .beat i:nth-child(2) { background: var(--c2); animation-delay: 0.15s; }
  .beat i:nth-child(3) { background: var(--c3); animation-delay: 0.3s; }
  .line.active .beat i { opacity: 1; }
  @keyframes bounce { 0%, 100% { transform: translateY(0); } 40% { transform: translateY(-8px); } }
  .plain { white-space: pre-wrap; font-size: 22px; font-weight: 600; line-height: 1.6; color: var(--text); margin: 0; }
  .state { font-size: 20px; font-weight: 600; color: var(--text-muted); margin: 0 0 16px; }
  .credit { margin: 32px 0 0; font-size: 12px; color: var(--text-faint); }

  .up-next {
    position: absolute; left: 48px; bottom: 28px; width: min(360px, calc(100% - 96px));
    display: grid; grid-template-columns: 56px 1fr auto auto; align-items: center; gap: 12px;
    padding: 12px 12px 14px; border-radius: 14px; overflow: hidden;
    background: color-mix(in srgb, var(--surface) 85%, transparent);
    border: 1px solid var(--border);
    box-shadow: 0 18px 48px rgba(0, 0, 0, 0.45), 0 0 40px color-mix(in srgb, var(--c1) 18%, transparent);
    backdrop-filter: blur(18px);
  }
  .up-next img { width: 56px; height: 56px; border-radius: 8px; object-fit: cover; }
  .next-text { min-width: 0; display: grid; gap: 2px; }
  .next-label { font-size: 11px; font-weight: 700; letter-spacing: 0.6px; text-transform: uppercase; color: var(--c1); }
  .next-title { font-weight: 600; font-size: 14px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .next-artist { font-size: 13px; color: var(--text-muted); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .up-next .play { background: var(--text); color: var(--bg); border-radius: 50%; }
  .up-next .play:hover { background: var(--text-muted); color: var(--bg); }
  .countdown {
    position: absolute; left: 0; bottom: 0; width: 100%; height: 3px;
    background: linear-gradient(90deg, var(--c1), var(--c2));
    transform-origin: left; transition: transform 0.3s linear;
  }

  @media (max-width: 860px) {
    .content { grid-template-columns: 1fr; padding: 16px 20px 0; gap: 0; }
    .track { display: none; }
  }
</style>
