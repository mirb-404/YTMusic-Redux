<script lang="ts">
  import type { Snippet } from "svelte";
  import { fade, fly } from "svelte/transition";
  import { cubicOut } from "svelte/easing";
  import Icon from "./Icon.svelte";
  import { bestThumbnail, formatTime, type PlayerState } from "../lib/api";
  import type { LyricsTrack } from "../lib/lyrics.svelte";

  let {
    player,
    lyrics,
    lineIndex,
    elapsed = 0,
    controls,
    drag,
    onclose,
    onseek
  }: {
    player: PlayerState | null;
    lyrics: LyricsTrack;
    lineIndex: number;
    /** Playback position in seconds, for the current line's fill. */
    elapsed?: number;
    /** Window controls, when this view is the window's only header. */
    controls?: Snippet;
    drag?: boolean;
    onclose: () => void;
    onseek: (seconds: number) => void;
  } = $props();

  const video = $derived(player?.video ?? null);
  const playing = $derived(player?.player.trackState === 1);
  const art = $derived(bestThumbnail(video?.thumbnails, 544));
  const synced = $derived(lyrics.data?.synced ?? null);

  // How far the current line has been sung, 0 to 1, from its start to the next line's.
  const lineProgress = $derived.by(() => {
    if (!synced || lineIndex < 0) return 0;
    const start = synced[lineIndex].time;
    const end = synced[lineIndex + 1]?.time ?? start + 4;
    return Math.max(0, Math.min(1, (elapsed - start) / Math.max(0.1, end - start)));
  });

  // Text size, remembered for this viewer.
  const SIZES = { s: "Small", m: "Medium", l: "Large" } as const;
  type Size = keyof typeof SIZES;
  const storedSize = (() => {
    try {
      return localStorage.getItem("lyrics-size") as Size | null;
    } catch {
      return null;
    }
  })();
  let size = $state<Size>(storedSize && storedSize in SIZES ? storedSize : "m");
  function setSize(next: Size) {
    size = next;
    try {
      localStorage.setItem("lyrics-size", next);
    } catch {}
  }

  let list = $state<HTMLElement>();
  // Scrolling by hand pauses following the song until "back to current" or a few seconds pass.
  let browsing = $state(false);
  let browseTimer: ReturnType<typeof setTimeout> | undefined;
  function browse() {
    browsing = true;
    clearTimeout(browseTimer);
    browseTimer = setTimeout(() => (browsing = false), 6000);
  }

  function follow(smooth = true) {
    const line = list?.querySelector<HTMLElement>(`[data-line="${Math.max(0, lineIndex)}"]`);
    line?.scrollIntoView({ block: "center", behavior: smooth ? "smooth" : "auto" });
  }

  function resume() {
    clearTimeout(browseTimer);
    browsing = false;
    follow();
  }

  $effect(() => {
    void lineIndex;
    if (list && synced && !browsing) follow();
  });

  // A new track's lyrics start in place, without a long scroll from the old position.
  $effect(() => {
    void synced;
    browsing = false;
    queueMicrotask(() => follow(false));
  });
</script>

<main class="now-playing size-{size}">
  {#if art}
    {#key art}<div class="backdrop" transition:fade={{ duration: 800 }}><img src={art} alt="" aria-hidden="true" /></div>{/key}
  {/if}
  <div class="shade" aria-hidden="true"></div>

  <header class:with-controls={!!controls} data-tauri-drag-region={controls ? drag : undefined}>
    <button class="round" title="Close (Esc)" onclick={onclose}><Icon name="chevronDown" size={22} /></button>
    <span class="label" data-tauri-drag-region={controls ? drag : undefined}>Lyrics</span>
    <span class="push" data-tauri-drag-region={controls ? drag : undefined}></span>
    <div class="sizes" role="group" aria-label="Text size">
      {#each Object.entries(SIZES) as [key, name] (key)}
        <button class:active={size === key} title="{name} text" onclick={() => setSize(key as Size)}><span class="aa aa-{key}">Aa</span></button>
      {/each}
    </div>
    {#if controls}<div class="controls">{@render controls()}</div>{/if}
  </header>

  <div class="content">
    {#key video?.id}
      <section class="track" in:fly={{ y: 16, duration: 450, easing: cubicOut }}>
        <div class="art" class:paused={!playing}>
          {#if art}<img src={art} alt="" />{:else}<Icon name="music" size={64} />{/if}
        </div>
        <h1 title={video?.title}>{video?.title ?? "Nothing playing"}</h1>
        <p>{video ? [video.author, video.album].filter(Boolean).join(" · ") : "Pick something to play"}</p>
      </section>
    {/key}

    <section class="lyrics" class:synced={!!synced && !lyrics.loading} bind:this={list} onwheel={browse}>
      {#if !video}
        <p class="state">Play a song to see its lyrics.</p>
      {:else if lyrics.loading}
        <div class="loading" aria-label="Loading lyrics">{#each [0, 1, 2, 3, 4] as i (i)}<span style:--i={i}></span>{/each}</div>
      {:else if lyrics.error}
        <p class="state">{lyrics.error}</p>
        <button class="btn" onclick={() => lyrics.load(video.id, true)}><Icon name="refresh" size={16} />Retry</button>
      {:else if synced}
        {#key synced}
          <div class="lines">
            {#each synced as line, i (i)}
              <button
                class="line"
                class:active={i === lineIndex}
                class:past={i < lineIndex}
                style:--i={Math.min(i, 12)}
                style:--depth={Math.min(4, Math.abs(i - lineIndex))}
                style:--p="{i === lineIndex ? lineProgress * 100 : i < lineIndex ? 100 : 0}%"
                data-line={i}
                onclick={() => {
                  resume();
                  onseek(line.time);
                }}
              >
                <span class="time">{formatTime(line.time)}</span>
                {#if line.text}<span class="text">{line.text}</span>{:else}<span class="beat"><i></i><i></i><i></i></span>{/if}
              </button>
            {/each}
          </div>
        {/key}
      {:else if lyrics.data?.plain}
        <p class="plain">{lyrics.data.plain}</p>
      {:else if lyrics.data?.instrumental}
        <p class="state">Instrumental</p>
      {:else}
        <p class="state">No lyrics found for this track.</p>
      {/if}
      {#if lyrics.data}<p class="credit">Lyrics from LRCLIB</p>{/if}
    </section>

    {#if browsing && synced}
      <button class="resume" onclick={resume} transition:fly={{ y: 12, duration: 220, easing: cubicOut }}>
        <Icon name="music" size={16} />Back to current line
      </button>
    {/if}
  </div>
</main>

<style>
  .now-playing {
    --lyric: 32px;
    position: relative; isolation: isolate; overflow: hidden;
    flex: 1; min-height: 0; display: flex; flex-direction: column;
    background: #000;
  }
  .size-s { --lyric: 24px; }
  .size-l { --lyric: 42px; }

  /* The artwork, blurred small and scaled up (cheap at any window size), drifting slowly. */
  .backdrop { position: absolute; inset: 0; z-index: -2; overflow: hidden; }
  .backdrop img {
    position: absolute; left: 50%; top: 50%; width: 30%; height: 30%;
    object-fit: cover;
    filter: blur(14px) saturate(1.6);
    transform: translate(-50%, -50%) scale(4.2);
    animation: drift 40s ease-in-out infinite alternate;
  }
  @keyframes drift {
    from { transform: translate(-50%, -50%) scale(4.2) rotate(0deg); }
    to { transform: translate(-46%, -53%) scale(4.6) rotate(8deg); }
  }
  .shade {
    position: absolute; inset: 0; z-index: -1;
    background: radial-gradient(ellipse at 30% 40%, rgba(0, 0, 0, 0.25), rgba(0, 0, 0, 0.7) 75%), rgba(0, 0, 0, 0.3);
  }

  header { display: flex; align-items: center; gap: 10px; padding: 12px 20px 0 14px; }
  /* The window controls sit flush with the window's top right corner. */
  header.with-controls { padding-right: 0; }
  .controls { align-self: flex-start; display: flex; align-items: center; gap: 2px; margin: -12px 0 0 8px; }
  .round {
    width: 36px; height: 36px; border-radius: 50%;
    display: grid; place-items: center;
    background: rgba(255, 255, 255, 0.1); color: #fff;
    transition: background var(--dur-fast) var(--ease), transform var(--dur-fast) var(--ease);
  }
  .round:hover { background: rgba(255, 255, 255, 0.18); transform: scale(1.06); }
  .round:active { transform: scale(0.94); }
  .label { font-size: 14px; font-weight: 700; color: rgba(255, 255, 255, 0.85); }
  .push { flex: 1; }
  .sizes { display: flex; gap: 2px; padding: 3px; border-radius: 999px; background: rgba(255, 255, 255, 0.08); }
  .sizes button {
    height: 28px; min-width: 36px; padding: 0 8px; border-radius: 999px;
    color: rgba(255, 255, 255, 0.6);
    transition: background var(--dur) var(--ease), color var(--dur) var(--ease);
  }
  .sizes button:hover { color: #fff; }
  .sizes button.active { background: rgba(255, 255, 255, 0.92); color: #000; }
  .aa { font-weight: 700; line-height: 1; }
  .aa-s { font-size: 11px; }
  .aa-m { font-size: 13px; }
  .aa-l { font-size: 16px; }

  .content {
    position: relative;
    flex: 1; min-height: 0;
    display: grid; grid-template-columns: minmax(240px, min(520px, 50vh)) minmax(0, 1fr);
    gap: clamp(32px, 5vw, 112px); padding: 8px clamp(24px, 5vw, 112px) 0;
    width: 100%; max-width: 1800px; margin: 0 auto;
  }
  .track { align-self: center; min-width: 0; padding-bottom: 40px; }
  .art {
    width: 100%; aspect-ratio: 1; border-radius: 10px; overflow: hidden;
    display: grid; place-items: center;
    background: rgba(255, 255, 255, 0.08); color: var(--text-faint);
    box-shadow: 0 30px 80px rgba(0, 0, 0, 0.55);
    transition: transform 0.6s var(--ease-spring), box-shadow 0.6s var(--ease);
  }
  /* Settles back when paused, the way a record player's platter stops. */
  .art.paused { transform: scale(0.9); box-shadow: 0 16px 40px rgba(0, 0, 0, 0.45); }
  .art img { width: 100%; height: 100%; object-fit: cover; }
  h1 { margin: 24px 0 4px; font-size: 26px; font-weight: 800; letter-spacing: -0.02em; line-height: 1.25; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .track p { margin: 0; color: rgba(255, 255, 255, 0.7); font-size: 15px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }

  .lyrics {
    overflow-y: auto; padding: 24px 8px 40px 0; scrollbar-width: none;
    /* Lines fade in and out at the edges instead of being cut. */
    mask-image: linear-gradient(to bottom, transparent 0, #000 12%, #000 82%, transparent 100%);
  }
  .lyrics.synced { padding: 38vh 8px 45vh 0; }
  .lyrics::-webkit-scrollbar { display: none; }

  .lines { display: grid; gap: 2px; }
  .line {
    position: relative;
    display: block; width: 100%; text-align: left;
    padding: 10px 16px; margin-left: -16px; border-radius: 12px;
    font-size: var(--lyric); font-weight: 800; line-height: 1.22; letter-spacing: -0.015em;
    color: rgba(255, 255, 255, calc(0.36 - var(--depth) * 0.05));
    transform-origin: left center;
    transform: scale(0.97);
    transition: color 0.45s var(--ease), transform 0.55s var(--ease-spring), background var(--dur) var(--ease), opacity 0.45s var(--ease);
    animation: line-in 0.5s var(--ease-out) both;
    animation-delay: calc(var(--i) * 25ms);
  }
  @keyframes line-in { from { opacity: 0; transform: translateY(12px) scale(0.97); } }
  .line.past { color: rgba(255, 255, 255, calc(0.28 - var(--depth) * 0.03)); }
  .line:hover { background: rgba(255, 255, 255, 0.06); color: rgba(255, 255, 255, 0.8); }
  /* The current line fills with white as it is sung, with a soft leading edge. */
  .line.active { transform: scale(1); }
  .line.active .text {
    background: linear-gradient(90deg, #fff calc(var(--p) - 8%), rgba(255, 255, 255, 0.4) calc(var(--p) + 8%));
    -webkit-background-clip: text;
    background-clip: text;
    color: transparent;
    -webkit-box-decoration-break: clone;
    box-decoration-break: clone;
  }
  /* Hovering a line shows where it starts; clicking seeks there. */
  .time {
    position: absolute; right: 16px; top: 50%; translate: 0 -50%;
    font-size: 12px; font-weight: 600; letter-spacing: 0; color: rgba(255, 255, 255, 0.6);
    font-variant-numeric: tabular-nums;
    opacity: 0; transition: opacity var(--dur) var(--ease);
  }
  .line:hover .time { opacity: 1; }

  /* Instrumental breaks: three dots breathing in turn. */
  .beat { display: inline-flex; gap: 8px; padding: 12px 0; }
  .beat i { width: 10px; height: 10px; border-radius: 50%; background: currentColor; animation: breathe 1.4s ease-in-out infinite; }
  .beat i:nth-child(2) { animation-delay: 0.2s; }
  .beat i:nth-child(3) { animation-delay: 0.4s; }
  .line.active .beat { color: #fff; }
  @keyframes breathe { 0%, 100% { transform: scale(0.7); opacity: 0.5; } 50% { transform: scale(1.1); opacity: 1; } }

  .plain { white-space: pre-wrap; font-size: calc(var(--lyric) * 0.7); font-weight: 700; line-height: 1.6; color: rgba(255, 255, 255, 0.9); margin: 0; }
  .state { font-size: 20px; font-weight: 700; color: rgba(255, 255, 255, 0.7); margin: 0 0 16px; }
  .credit { margin: 32px 0 0; font-size: 12px; color: rgba(255, 255, 255, 0.45); }

  .loading { display: grid; gap: 18px; padding-top: 30vh; }
  .loading span {
    display: block; height: calc(var(--lyric) * 0.8); border-radius: 10px;
    width: calc(85% - var(--i) * 9%);
    background: rgba(255, 255, 255, 0.08);
    animation: pulse 1.4s ease-in-out infinite;
    animation-delay: calc(var(--i) * 0.12s);
  }
  @keyframes pulse { 50% { opacity: 0.4; } }

  .resume {
    position: absolute; right: clamp(24px, 5vw, 112px); bottom: 28px;
    display: inline-flex; align-items: center; gap: 8px;
    height: 38px; padding: 0 18px; border-radius: 999px;
    background: #fff; color: #000; font-weight: 700; font-size: 13.5px;
    box-shadow: 0 10px 30px rgba(0, 0, 0, 0.45);
    transition: transform var(--dur-fast) var(--ease);
  }
  .resume:hover { transform: scale(1.04); }
  .resume:active { transform: scale(0.97); }

  @media (max-width: 860px) {
    .content { grid-template-columns: 1fr; padding: 8px 20px 0; gap: 0; }
    .track { display: none; }
  }
</style>
