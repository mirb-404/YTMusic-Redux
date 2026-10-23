<script lang="ts">
  import Icon from "../Icon.svelte";
  import Card from "./Card.svelte";
  import TrackRow from "./TrackRow.svelte";
  import { router, type Section } from "../../lib/music.svelte";

  // A horizontal row of cards (or of track columns), scrolled by its arrows or the wheel.
  let { section }: { section: Section } = $props();

  let track = $state<HTMLElement>();
  let atStart = $state(true);
  let atEnd = $state(false);

  function update() {
    if (!track) return;
    atStart = track.scrollLeft < 4;
    atEnd = track.scrollLeft + track.clientWidth > track.scrollWidth - 4;
  }

  function page(direction: number) {
    track?.scrollBy({ left: direction * track.clientWidth * 0.85, behavior: "smooth" });
  }

  $effect(() => {
    section.items.length;
    queueMicrotask(update);
  });

  // Window resizes, and a shelf first laid out as it scrolls into view, change what fits.
  $effect(() => {
    if (!track) return;
    const observer = new ResizeObserver(update);
    observer.observe(track);
    return () => observer.disconnect();
  });
</script>

<section class="shelf">
  <header>
    <div class="titles">
      {#if section.strapline}<div class="strapline">{section.strapline}</div>{/if}
      <h2>{section.title}</h2>
    </div>
    <div class="actions">
      {#if section.more}<button class="more" onclick={() => router.open(section.more)}>Show all</button>{/if}
      <button class="nav" title="Previous" disabled={atStart} onclick={() => page(-1)}><Icon name="chevronLeft" size={20} /></button>
      <button class="nav" title="Next" disabled={atEnd} onclick={() => page(1)}><Icon name="chevronRight" size={20} /></button>
    </div>
  </header>
  <div class="track" class:rows={section.rows} bind:this={track} onscroll={update}>
    {#if section.rows}
      {#each section.items as item, i (i)}<div class="row-cell"><TrackRow {item} /></div>{/each}
    {:else}
      {#each section.items as item, i (i)}<Card {item} />{/each}
    {/if}
  </div>
</section>

<style>
  .shelf { margin-bottom: 36px; }
  header { display: flex; align-items: flex-end; justify-content: space-between; gap: 16px; margin-bottom: 12px; }
  .strapline { font-size: 13px; font-weight: 500; color: var(--text-muted); margin-bottom: 2px; }
  h2 { margin: 0; font-size: 24px; font-weight: 800; letter-spacing: -0.02em; }
  .actions { display: flex; align-items: center; gap: 8px; }
  .more {
    padding: 0 4px; margin-right: 6px;
    font-size: 14px; font-weight: 700; color: var(--text-muted);
    transition: color var(--dur-fast) var(--ease);
  }
  .more:hover { color: var(--text); text-decoration: underline; }
  .nav {
    width: 32px; height: 32px; border-radius: 50%;
    display: grid; place-items: center;
    background: var(--bg-elevated); color: var(--text);
    transition: background var(--dur-fast) var(--ease), opacity var(--dur) var(--ease), transform var(--dur-fast) var(--ease);
  }
  .nav:hover:not(:disabled) { background: var(--bg-highlight); transform: scale(1.06); }
  .nav:active:not(:disabled) { transform: scale(0.94); }
  .nav:disabled { opacity: 0.3; cursor: default; }
  .track {
    display: flex; gap: 0;
    overflow-x: auto; overflow-y: hidden;
    padding: 12px 0; margin: 0 -12px;
    scroll-snap-type: x proximity;
    scroll-padding: 0 12px;
    scrollbar-width: none;
  }
  .track::-webkit-scrollbar { display: none; }
  .track > :global(*) { scroll-snap-align: start; }
  .track.rows {
    display: grid; grid-auto-flow: column; grid-template-rows: repeat(4, auto);
    grid-auto-columns: minmax(340px, 30%); gap: 0 16px; padding: 0 12px;
  }
</style>
