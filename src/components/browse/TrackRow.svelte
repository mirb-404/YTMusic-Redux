<script lang="ts">
  import Icon from "../Icon.svelte";
  import Runs from "./Runs.svelte";
  import { art, nowPlaying, router, type Item } from "../../lib/music.svelte";

  // A song, video or result row. Albums show their track numbers instead of artwork.
  let { item, numbered = false }: { item: Item; numbered?: boolean } = $props();

  const src = $derived(numbered ? null : art(item.thumbnails, 44));
  const current = $derived(!!item.videoId && item.videoId === nowPlaying.id);
</script>

<div
  class="row"
  class:current
  role="button"
  tabindex="0"
  onclick={() => router.open(item.endpoint)}
  onkeydown={(e) => e.key === "Enter" && router.open(item.endpoint)}
>
  <div class="lead" class:numbered>
    {#if numbered}
      <span class="num">{item.index ?? ""}</span>
    {:else}
      <span class="thumb" class:circle={item.aspect === "circle"}>{#if src}<img {src} alt="" loading="lazy" decoding="async" />{/if}</span>
    {/if}
    {#if current && nowPlaying.playing}
      <span class="eq" aria-hidden="true"><i></i><i></i><i></i></span>
    {:else}
      <span class="hover-play"><Icon name="play" size={18} /></span>
    {/if}
  </div>
  <div class="meta">
    <div class="title" title={item.title}>{item.title}</div>
    {#if item.subtitle.length}<div class="sub"><Runs runs={item.subtitle} /></div>{/if}
  </div>
  {#if item.duration}<span class="duration">{item.duration}</span>{/if}
</div>

<style>
  .row {
    display: grid; grid-template-columns: 44px minmax(0, 1fr) auto; align-items: center; gap: 14px;
    height: 60px; padding: 0 14px 0 8px;
    border-radius: 8px; cursor: pointer;
    transition: background 0.15s var(--ease);
  }
  .row:hover, .row:focus-visible { background: var(--surface-hover); }
  .row:focus-visible { outline: 2px solid var(--accent); outline-offset: -2px; }
  .lead { position: relative; width: 44px; height: 44px; display: grid; place-items: center; }
  .thumb { width: 44px; height: 44px; border-radius: 6px; overflow: hidden; background: var(--surface-hover); }
  .thumb.circle { border-radius: 50%; }
  .thumb img { width: 100%; height: 100%; object-fit: cover; }
  .num { color: var(--text-muted); font-variant-numeric: tabular-nums; font-size: 14px; }
  .hover-play, .eq {
    position: absolute; inset: 0; border-radius: 6px;
    display: grid; place-items: center;
    color: #fff;
  }
  .lead:not(.numbered) .hover-play, .lead:not(.numbered) .eq { background: rgba(0, 0, 0, 0.55); }
  .numbered .hover-play, .numbered .eq { background: var(--bg-elevated); }
  .hover-play { opacity: 0; transition: opacity 0.15s; }
  .row:hover .hover-play { opacity: 1; }
  .eq { display: flex; align-items: flex-end; justify-content: center; gap: 3px; padding-bottom: 14px; }
  .eq i { width: 3px; height: 14px; border-radius: 2px; background: var(--accent); animation: eq 0.9s ease-in-out infinite; transform-origin: bottom; }
  .eq i:nth-child(2) { animation-delay: -0.3s; }
  .eq i:nth-child(3) { animation-delay: -0.6s; }
  @keyframes eq { 0%, 100% { transform: scaleY(0.3); } 50% { transform: scaleY(1); } }
  .meta { min-width: 0; }
  .title { font-weight: 600; font-size: 14px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .current .title { color: var(--accent); }
  .sub { font-size: 13px; color: var(--text-muted); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; margin-top: 2px; }
  .duration { font-size: 13px; color: var(--text-muted); font-variant-numeric: tabular-nums; }
</style>
