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
  .card {
    width: var(--w); flex: none;
    padding: 10px; margin: -10px;
    border-radius: 12px;
    cursor: pointer;
    transition: background 0.2s var(--ease);
  }
  .card:hover, .card:focus-visible { background: var(--surface); }
  .card:focus-visible { outline: 2px solid var(--accent); outline-offset: 0; }
  .art {
    position: relative;
    width: 100%; aspect-ratio: 1;
    border-radius: 8px; overflow: hidden;
    display: grid; place-items: center;
    background: var(--surface-hover); color: var(--text-faint);
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.35);
  }
  .art.wide { aspect-ratio: 16 / 9; }
  .art.circle { border-radius: 50%; }
  .art img { width: 100%; height: 100%; object-fit: cover; transition: transform 0.45s var(--ease); }
  .card:hover .art img { transform: scale(1.04); }
  .play {
    position: absolute; right: 10px; bottom: 10px;
    width: 42px; height: 42px; border-radius: 50%;
    display: grid; place-items: center;
    background: #fff; color: #000;
    box-shadow: 0 6px 18px rgba(0, 0, 0, 0.45);
    opacity: 0; transform: translateY(8px) scale(0.9);
    transition: opacity 0.2s var(--ease), transform 0.25s var(--ease), background 0.15s;
  }
  .circle .play { right: 50%; bottom: 50%; margin: 0 -21px -21px 0; }
  .card:hover .play, .play.on { opacity: 1; transform: none; }
  .play:hover { transform: scale(1.08) !important; }
  .play.on { background: var(--accent); color: #fff; }
  .more {
    position: absolute; top: 8px; right: 8px; z-index: 1;
    width: 32px; height: 32px; border-radius: 50%;
    display: grid; place-items: center;
    background: rgba(0, 0, 0, 0.55); color: #fff; backdrop-filter: blur(8px);
    opacity: 0; transition: opacity 0.2s, background 0.15s;
  }
  .card:hover .more { opacity: 1; }
  .more:hover { background: rgba(0, 0, 0, 0.8); }
  .circle .more { top: 4px; right: 4px; }
  .title { margin-top: 10px; font-weight: 600; font-size: 14px; line-height: 1.3; display: -webkit-box; -webkit-line-clamp: 2; line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; }
  .sub { margin-top: 3px; font-size: 13px; color: var(--text-muted); display: -webkit-box; -webkit-line-clamp: 2; line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; }
  .circle ~ .title, .circle ~ .sub { text-align: center; }

  .category {
    flex: none; width: 200px; height: 56px;
    padding: 0 16px; border-radius: 10px;
    text-align: left; font-weight: 600; font-size: 14px;
    background: linear-gradient(90deg, color-mix(in srgb, var(--stripe) 28%, transparent), var(--surface) 70%);
    border-left: 4px solid var(--stripe);
    transition: transform 0.2s var(--ease), background 0.2s;
  }
  .category:hover { transform: translateY(-2px); background: linear-gradient(90deg, color-mix(in srgb, var(--stripe) 40%, transparent), var(--surface-hover) 70%); }
</style>
