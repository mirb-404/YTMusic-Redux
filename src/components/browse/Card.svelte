<script lang="ts">
  import Icon from "../Icon.svelte";
  import Runs from "./Runs.svelte";
  import { art, music, nowPlaying, openMenu, router, type Item } from "../../lib/music.svelte";

  let { item, width = 180 }: { item: Item; width?: number } = $props();

  const wide = $derived(item.aspect === "wide");
  const src = $derived(art(item.thumbnails, wide ? width : width));
  const playEndpoint = $derived(item.play ?? (item.endpoint?.watchEndpoint || item.endpoint?.watchPlaylistEndpoint ? item.endpoint : null));
  const isPlaying = $derived(!!item.videoId && item.videoId === nowPlaying.id);

  function play(e: Event) {
    e.stopPropagation();
    if (playEndpoint) music.play(playEndpoint);
  }
</script>

{#if item.kind === "category"}
  <button class="category" style:--stripe={item.color ?? "var(--accent)"} onclick={() => router.open(item.endpoint)}>{item.title}</button>
{:else}
  <div
    class="card"
    class:wide
    style:--w="{wide ? Math.round(width * 1.6) : width}px"
    role="button"
    tabindex="0"
    onclick={() => router.open(item.endpoint)}
    onkeydown={(e) => e.key === "Enter" && router.open(item.endpoint)}
    oncontextmenu={(e) => openMenu(e, item.menu, item)}
  >
    <div class="art" class:circle={item.aspect === "circle"} class:wide>
      {#if item.menu.length}
        <button class="more" title="More" tabindex="-1" onclick={(e) => openMenu(e, item.menu, item)}><Icon name="more" size={20} /></button>
      {/if}
      {#if src}<img {src} alt="" loading="lazy" decoding="async" />{:else}<Icon name="music" size={32} />{/if}
      {#if playEndpoint}
        <button class="play" class:on={isPlaying} title="Play" tabindex="-1" onclick={play}><Icon name={isPlaying ? "volume" : "play"} size={20} /></button>
      {/if}
    </div>
    <div class="title" title={item.title}>{item.title}</div>
    {#if item.subtitle.length}<div class="sub"><Runs runs={item.subtitle} /></div>{/if}
  </div>
{/if}

<style>
  /* Spotify's card: a quiet tile that lifts on hover, its play button rising from the corner. */
  .card {
    width: var(--w); flex: none;
    padding: 12px; margin: -12px 0;
    border-radius: var(--radius);
    cursor: pointer;
    transition: background var(--dur-slow) var(--ease);
  }
  .card:hover, .card:focus-visible { background: var(--bg-elevated); }
  .card:active { background: #000; }
  .card:focus-visible { outline: 2px solid var(--text); outline-offset: 0; }
  .art {
    position: relative;
    width: 100%; aspect-ratio: 1;
    border-radius: var(--radius-sm); overflow: hidden;
    display: grid; place-items: center;
    background: var(--bg-highlight); color: var(--text-faint);
    box-shadow: var(--shadow-card);
  }
  .art.wide { aspect-ratio: 16 / 9; }
  .art.circle { border-radius: 50%; }
  .art img { width: 100%; height: 100%; object-fit: cover; animation: fade-in var(--dur-slow) var(--ease) both; }
  @keyframes fade-in { from { opacity: 0; } }
  .play {
    position: absolute; right: 8px; bottom: 8px;
    width: 48px; height: 48px; border-radius: 50%;
    display: grid; place-items: center;
    background: var(--accent); color: var(--accent-ink);
    box-shadow: 0 8px 8px rgba(0, 0, 0, 0.3);
    opacity: 0; transform: translateY(8px);
    transition: opacity var(--dur-slow) var(--ease), transform var(--dur-slow) var(--ease), background var(--dur-fast) var(--ease);
  }
  .circle .play { right: 4px; bottom: 4px; }
  .card:hover .play, .card:focus-visible .play, .play.on { opacity: 1; transform: none; }
  .play:hover { transform: scale(1.06) !important; background: var(--accent-hover); }
  .play:active { transform: scale(0.96) !important; }
  .more {
    position: absolute; top: 8px; right: 8px; z-index: 1;
    width: 32px; height: 32px; border-radius: 50%;
    display: grid; place-items: center;
    background: rgba(0, 0, 0, 0.6); color: #fff; backdrop-filter: blur(8px);
    opacity: 0; transition: opacity var(--dur) var(--ease), background var(--dur-fast) var(--ease), transform var(--dur-fast) var(--ease);
  }
  .card:hover .more { opacity: 1; }
  .more:hover { background: rgba(0, 0, 0, 0.85); transform: scale(1.06); }
  .circle .more { top: 4px; right: 4px; }
  .title { margin-top: 10px; font-weight: 600; font-size: 15px; line-height: 1.3; display: -webkit-box; -webkit-line-clamp: 2; line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; }
  .sub { margin-top: 4px; font-size: 13.5px; color: var(--text-muted); display: -webkit-box; -webkit-line-clamp: 2; line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; }
  .circle ~ .title, .circle ~ .sub { text-align: left; }

  /* Explore's moods and genres: Spotify's bold color blocks. */
  .category {
    position: relative; overflow: hidden;
    flex: none; width: 200px; height: 96px; margin-left: 12px;
    padding: 14px 16px; border-radius: var(--radius);
    text-align: left; align-content: start; display: grid;
    font-weight: 800; font-size: 18px; line-height: 1.2; letter-spacing: -0.01em;
    background: linear-gradient(135deg, var(--stripe), color-mix(in srgb, var(--stripe) 45%, #000));
    box-shadow: var(--shadow-card);
    transition: transform var(--dur) var(--ease), filter var(--dur) var(--ease);
  }
  .category::after {
    content: ""; position: absolute; right: -18px; bottom: -18px; width: 72px; height: 72px; border-radius: 10px;
    background: rgba(255, 255, 255, 0.16); transform: rotate(25deg);
    transition: transform var(--dur-slow) var(--ease-out);
  }
  .category:hover { transform: scale(1.03); filter: brightness(1.08); }
  .category:hover::after { transform: rotate(32deg) translate(-4px, -4px); }
  .category:active { transform: scale(0.98); }
</style>
