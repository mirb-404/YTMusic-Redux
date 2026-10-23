<script lang="ts">
  import { fly } from "svelte/transition";
  import { cubicOut } from "svelte/easing";
  import Icon from "../Icon.svelte";
  import { api, type PlayerState, type QueueItem } from "../../lib/api";
  import { art, music, nowPlaying, toast, ui } from "../../lib/music.svelte";

  // The play queue beside the pages: YTM's own queue, played, reordered and trimmed from here.
  let { player }: { player: PlayerState | null } = $props();

  const queue = $derived(player?.player.queue ?? null);
  const items = $derived(queue?.items ?? []);
  const automix = $derived(queue?.automixItems ?? []);
  const current = $derived(queue?.selectedItemIndex ?? -1);

  let list = $state<HTMLElement>();
  let dragFrom = $state<number | null>(null);
  let dragOver = $state<number | null>(null);

  // Opening scrolls the playing track into view.
  $effect(() => {
    if (!list || current < 0) return;
    list.querySelector<HTMLElement>(`[data-index="${current}"]`)?.scrollIntoView({ block: "center" });
  });

  function play(index: number) {
    api.command("playQueueIndex", index).catch((e) => toast(String(e)));
  }

  function remove(e: Event, index: number) {
    e.stopPropagation();
    music.queueRemove(index).catch((err) => toast(String(err)));
  }

  function drop(to: number) {
    const from = dragFrom;
    dragFrom = dragOver = null;
    if (from === null || from === to) return;
    music.queueMove(from, to).catch((e) => toast(String(e)));
  }
</script>

{#snippet row(item: QueueItem, index: number, editable: boolean)}
  <div
    class="item"
    class:current={index === current}
    class:over={dragOver === index && dragFrom !== index}
    data-index={index}
    role="button"
    tabindex="0"
    draggable={editable}
    ondragstart={() => (dragFrom = index)}
    ondragover={(e) => {
      if (!editable || dragFrom === null) return;
      e.preventDefault();
      dragOver = index;
    }}
    ondragend={() => (dragFrom = dragOver = null)}
    ondrop={() => drop(index)}
    onclick={() => play(index)}
    onkeydown={(e) => e.key === "Enter" && play(index)}
  >
    <span class="thumb">
      {#if art(item.thumbnails, 44)}<img src={art(item.thumbnails, 44)} alt="" loading="lazy" />{/if}
      {#if index === current}
        <span class="eq" class:paused={!nowPlaying.playing}><i></i><i></i><i></i></span>
      {/if}
    </span>
    <span class="meta">
      <span class="title">{item.title}</span>
      <span class="author">{item.author}</span>
    </span>
    <span class="duration">{item.duration}</span>
    {#if editable && index !== current}
      <button class="remove" title="Remove from queue" onclick={(e) => remove(e, index)}><Icon name="close" size={16} /></button>
    {/if}
  </div>
{/snippet}

{#if ui.queueOpen}
  <aside class="panel" transition:fly={{ x: 48, duration: 280, easing: cubicOut }}>
    <div class="inner">
    <header>
      <h2>Queue</h2>
      <button class="x" title="Close (Q)" onclick={() => (ui.queueOpen = false)}><Icon name="close" size={18} /></button>
    </header>
    <div class="list" bind:this={list}>
      {#if !items.length}
        <p class="empty">Nothing in the queue. Play something to start one.</p>
      {/if}
      {#each items as item, i (i)}{@render row(item, i, true)}{/each}
      {#if automix.length}
        <div class="label">Autoplay</div>
        {#each automix as item, i (i)}{@render row(item, items.length + i, false)}{/each}
      {/if}
    </div>
    </div>
  </aside>
{/if}

<style>
  /* Docked beside the page like Spotify's queue. It slides by transform: animating its width
     would reflow the whole page every frame. */
  .panel { flex: none; width: 360px; min-height: 0; border-radius: var(--radius-panel); background: var(--bg-panel); overflow: hidden; }
  .inner { width: 360px; height: 100%; display: flex; flex-direction: column; }
  header { display: flex; align-items: center; justify-content: space-between; padding: 16px 12px 8px 16px; }
  h2 { margin: 0; font-size: 16px; font-weight: 700; }
  .x {
    width: 32px; height: 32px; border-radius: 50%; display: grid; place-items: center; color: var(--text-muted);
    transition: background var(--dur-fast) var(--ease), color var(--dur-fast) var(--ease), transform var(--dur-fast) var(--ease);
  }
  .x:hover { background: var(--bg-elevated); color: var(--text); transform: scale(1.06); }
  .list { flex: 1; min-height: 0; overflow-y: auto; padding: 0 8px 16px; }
  .list::-webkit-scrollbar { width: 8px; }
  .empty { padding: 24px 8px; color: var(--text-muted); font-size: 14px; }
  .label { margin: 20px 8px 6px; font-size: 16px; font-weight: 700; color: var(--text); }
  .item {
    position: relative; display: grid; grid-template-columns: 48px minmax(0, 1fr) auto auto; align-items: center; gap: 12px;
    padding: 8px; border-radius: var(--radius-sm); cursor: pointer;
    transition: background var(--dur-fast) var(--ease);
  }
  .item:hover { background: var(--surface-hover); }
  .item:active { background: var(--surface-active); }
  .item.current { background: color-mix(in srgb, var(--accent) 12%, transparent); }
  .item.over { box-shadow: inset 0 2px 0 var(--accent); }
  .thumb { position: relative; width: 48px; height: 48px; border-radius: 4px; overflow: hidden; background: var(--bg-highlight); }
  .thumb img { width: 100%; height: 100%; object-fit: cover; }
  .eq { position: absolute; inset: 0; display: flex; align-items: flex-end; justify-content: center; gap: 3px; padding-bottom: 15px; background: rgba(0, 0, 0, 0.55); }
  .eq i { width: 3px; height: 14px; border-radius: 1px; background: var(--accent); animation: eq 0.9s ease-in-out infinite; transform-origin: bottom; }
  .eq i:nth-child(2) { animation-delay: -0.3s; }
  .eq i:nth-child(3) { animation-delay: -0.6s; }
  .eq.paused i { animation-play-state: paused; }
  @keyframes eq { 0%, 100% { transform: scaleY(0.3); } 50% { transform: scaleY(1); } }
  .meta { min-width: 0; display: grid; gap: 1px; }
  .title { font-weight: 500; font-size: 15px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .current .title { color: var(--accent); }
  .author { font-size: 13.5px; color: var(--text-muted); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .duration { font-size: 13px; color: var(--text-muted); font-variant-numeric: tabular-nums; }
  .remove {
    width: 28px; height: 28px; border-radius: 50%; display: grid; place-items: center; color: var(--text-muted);
    opacity: 0; transition: opacity var(--dur-fast) var(--ease), color var(--dur-fast) var(--ease), transform var(--dur-fast) var(--ease);
  }
  .item:hover .remove { opacity: 1; }
  .remove:hover { color: var(--text); transform: scale(1.1); }
  @media (max-width: 1100px) { .panel, .inner { width: 300px; } }
</style>
