<script lang="ts">
  import Icon from "../Icon.svelte";
  import Runs from "./Runs.svelte";
  import { art, nowPlaying, openMenu, rate, router, type Item } from "../../lib/music.svelte";

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
  oncontextmenu={(e) => openMenu(e, item.menu, item)}
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
  <div class="tail">
    {#if item.like}
      <button
        class="icon"
        class:liked={item.like.status === "LIKE"}
        title={item.like.status === "LIKE" ? "Remove from Liked music" : "Like"}
        onclick={(e) => {
          e.stopPropagation();
          rate(item.like!, item.like!.status === "LIKE" ? "INDIFFERENT" : "LIKE");
        }}><Icon name={item.like.status === "LIKE" ? "like" : "likeOutline"} size={18} /></button
      >
    {/if}
    {#if item.duration}<span class="duration">{item.duration}</span>{/if}
    {#if item.menu.length}
      <button class="icon more" title="More" onclick={(e) => openMenu(e, item.menu, item)}><Icon name="more" size={20} /></button>
    {/if}
  </div>
</div>

<style>
  .row {
    display: grid; grid-template-columns: 40px minmax(0, 1fr) auto; align-items: center; gap: 14px;
    height: 56px; padding: 0 16px 0 8px;
    border-radius: 4px; cursor: pointer;
    transition: background var(--dur-fast) var(--ease);
  }
  .row:hover, .row:focus-visible { background: var(--surface-hover); }
  .row:active { background: var(--surface-active); }
  .row:focus-visible { outline: 2px solid var(--text); outline-offset: -2px; }
  .lead { position: relative; width: 40px; height: 40px; display: grid; place-items: center; }
  .thumb { width: 40px; height: 40px; border-radius: 4px; overflow: hidden; background: var(--bg-highlight); }
  .thumb.circle { border-radius: 50%; }
  .thumb img { width: 100%; height: 100%; object-fit: cover; }
  .num { color: var(--text-muted); font-variant-numeric: tabular-nums; font-size: 15px; transition: opacity var(--dur-fast) var(--ease); }
  .current .num { color: var(--accent); }
  .hover-play, .eq {
    position: absolute; inset: 0; border-radius: 4px;
    display: grid; place-items: center;
    color: #fff;
  }
  .lead:not(.numbered) .hover-play, .lead:not(.numbered) .eq { background: rgba(0, 0, 0, 0.55); }
  .hover-play { opacity: 0; transition: opacity var(--dur-fast) var(--ease); }
  .row:hover .hover-play { opacity: 1; }
  .numbered .hover-play { background: none; }
  .row:hover .numbered .num { opacity: 0; }
  .numbered .eq { background: transparent; }
  .row:has(.eq) .num { opacity: 0; }
  .eq { display: flex; align-items: flex-end; justify-content: center; gap: 3px; padding-bottom: 13px; }
  .eq i { width: 3px; height: 14px; border-radius: 1px; background: var(--accent); animation: eq 0.9s ease-in-out infinite; transform-origin: bottom; }
  .eq i:nth-child(2) { animation-delay: -0.3s; }
  .eq i:nth-child(3) { animation-delay: -0.6s; }
  @keyframes eq { 0%, 100% { transform: scaleY(0.3); } 50% { transform: scaleY(1); } }
  .meta { min-width: 0; }
  .title { font-weight: 500; font-size: 15px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .current .title { color: var(--accent); }
  .sub { font-size: 13.5px; color: var(--text-muted); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; margin-top: 1px; }
  .row:hover .sub { color: var(--text); }
  .duration { font-size: 13.5px; color: var(--text-muted); font-variant-numeric: tabular-nums; min-width: 38px; text-align: right; }
  .tail { display: flex; align-items: center; gap: 8px; }
  .icon {
    width: 32px; height: 32px; border-radius: 50%; display: grid; place-items: center; color: var(--text-muted);
    opacity: 0; transition: opacity var(--dur-fast) var(--ease), color var(--dur-fast) var(--ease), transform var(--dur-fast) var(--ease);
  }
  .row:hover .icon, .row:focus-within .icon, .icon.liked { opacity: 1; }
  .icon:hover { color: var(--text); transform: scale(1.08); }
  .icon:active { transform: scale(0.92); }
  .icon.liked { color: var(--accent); animation: pop 0.35s var(--ease-spring); }
  @keyframes pop { 40% { transform: scale(1.25); } }
</style>
