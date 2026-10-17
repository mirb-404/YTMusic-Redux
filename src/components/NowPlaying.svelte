<script lang="ts">
  import Icon from "./Icon.svelte";
  import { api, bestThumbnail, type PlayerState } from "../lib/api";
  import type { LyricsTrack } from "../lib/lyrics.svelte";

  let {
    player,
    lyrics,
    lineIndex,
    captions,
    showVideos,
    onclose,
    onseek
  }: {
    player: PlayerState | null;
    lyrics: LyricsTrack;
    lineIndex: number;
    captions: boolean;
    showVideos: boolean;
    onclose: () => void;
    onseek: (seconds: number) => void;
  } = $props();

  const video = $derived(player?.video ?? null);
  const art = $derived(bestThumbnail(video?.thumbnails, 544));
  const synced = $derived(lyrics.data?.synced ?? null);

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

<main class="now-playing">
  <header>
    <button class="icon-btn" title="Close (Esc)" onclick={onclose}><Icon name="chevronDown" size={22} /></button>
    <span class="label">Now playing</span>
    <span class="push"></span>
    <button class="chip" class:on={captions} title="Show the current lyric line over YouTube Music" onclick={() => api.setSetting("appearance.lyricsCaptions", !captions)}>
      <Icon name="captions" size={16} />Captions
    </button>
    <button class="chip" class:on={showVideos} title={showVideos ? "Playing music videos" : "Playing audio only"} onclick={() => api.setSetting("playback.showVideos", !showVideos)}>
      <Icon name={showVideos ? "video" : "videoOff"} size={16} />{showVideos ? "Video" : "Audio only"}
    </button>
    <button class="chip" title="Equalizer and playback" onclick={() => api.openSettings("audio")}><Icon name="equalizer" size={16} />Audio</button>
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
          <button class="line" class:active={i === lineIndex} class:past={i < lineIndex} data-line={i} onclick={() => onseek(line.time)}>
            {line.text || "♪"}
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
</main>

<style>
  .now-playing { flex: 1; min-height: 0; display: flex; flex-direction: column; background: var(--bg); }
  header { display: flex; align-items: center; gap: 8px; padding: 12px 20px 0 14px; }
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
    padding: 6px 0; font-size: 28px; font-weight: 700; line-height: 1.3;
    color: rgba(255, 255, 255, 0.32);
    transition: color 0.2s;
  }
  .line.past { color: rgba(255, 255, 255, 0.5); }
  .line.active { color: var(--text); }
  .line:hover { color: rgba(255, 255, 255, 0.8); }
  .plain { white-space: pre-wrap; font-size: 22px; font-weight: 600; line-height: 1.6; color: var(--text); margin: 0; }
  .state { font-size: 20px; font-weight: 600; color: var(--text-muted); margin: 0 0 16px; }
  .credit { margin: 32px 0 0; font-size: 12px; color: var(--text-faint); }

  @media (max-width: 860px) {
    .content { grid-template-columns: 1fr; padding: 16px 20px 0; gap: 0; }
    .track { display: none; }
  }
</style>
