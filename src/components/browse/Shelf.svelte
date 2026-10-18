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
  .shelf { margin-bottom: 40px; }
  header { display: flex; align-items: flex-end; justify-content: space-between; gap: 16px; margin-bottom: 16px; }
  .strapline { font-size: 12px; font-weight: 600; letter-spacing: 0.6px; text-transform: uppercase; color: var(--text-faint); margin-bottom: 2px; }
  h2 { margin: 0; font-size: 22px; font-weight: 700; letter-spacing: -0.01em; }
  .actions { display: flex; align-items: center; gap: 6px; }
  .more {
    height: 32px; padding: 0 14px; border-radius: 16px;
    font-size: 13px; font-weight: 600; color: var(--text-muted);
    border: 1px solid var(--border);
    transition: background 0.15s, color 0.15s;
  }
  .more:hover { background: var(--surface-hover); color: var(--text); }
  .nav {
    width: 32px; height: 32px; border-radius: 50%;
    display: grid; place-items: center;
    border: 1px solid var(--border); color: var(--text);
    transition: background 0.15s, opacity 0.15s;
  }
  .nav:hover:not(:disabled) { background: var(--surface-hover); }
  .nav:disabled { opacity: 0.3; cursor: default; }
  .track {
    display: flex; gap: 20px;
    overflow-x: auto; overflow-y: hidden;
    padding: 10px; margin: -10px;
    scroll-snap-type: x proximity;
    scrollbar-width: none;
  }
  .track::-webkit-scrollbar { display: none; }
  .track > :global(*) { scroll-snap-align: start; }
  .track.rows {
    display: grid; grid-auto-flow: column; grid-template-rows: repeat(4, auto);
    grid-auto-columns: minmax(340px, 30%); gap: 2px 16px;
  }
</style>
