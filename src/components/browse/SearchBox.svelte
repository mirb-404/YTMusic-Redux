<script lang="ts">
  import { fade } from "svelte/transition";
  import Icon from "../Icon.svelte";
  import Runs from "./Runs.svelte";
  import { art, music, router, type Item, type Run, type Suggestions } from "../../lib/music.svelte";

  let input = $state<HTMLInputElement>();
  let query = $state(router.current?.route.kind === "search" ? router.current.route.query : "");
  let suggestions = $state<Suggestions>({ queries: [], items: [] });
  let open = $state(false);
  let active = $state(-1);
  let timer: ReturnType<typeof setTimeout> | undefined;
  let requested = "";

  const text = (runs: Run[]) => runs.map((r) => r.text).join("");
  const count = $derived(suggestions.queries.length + suggestions.items.length);

  export function focus() {
    input?.focus();
    input?.select();
  }

  function onInput() {
    active = -1;
    open = true;
    clearTimeout(timer);
    const q = query.trim();
    if (!q) {
      suggestions = { queries: [], items: [] };
      return;
    }
    timer = setTimeout(async () => {
      requested = q;
      const s = await music.suggestions(q).catch(() => null);
      // A slower answer for an older query is dropped.
      if (s && requested === q) suggestions = s;
    }, 120);
  }

  function search(q: string) {
    q = q.trim();
    if (!q) return;
    query = q;
    open = false;
    input?.blur();
    router.go({ kind: "search", query: q });
  }

  function pick(item: Item) {
    open = false;
    input?.blur();
    router.open(item.endpoint);
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault();
      if (!count) return;
      // -1 is the typed query itself.
      const next = active + (e.key === "ArrowDown" ? 1 : -1);
      active = next < -1 ? count - 1 : next >= count ? -1 : next;
    } else if (e.key === "Enter") {
      e.preventDefault();
      if (active >= 0 && active < suggestions.queries.length) search(text(suggestions.queries[active]));
      else if (active >= suggestions.queries.length) pick(suggestions.items[active - suggestions.queries.length]);
      else search(query);
    } else if (e.key === "Escape") {
      e.stopPropagation();
      if (open && query) open = false;
      else input?.blur();
    }
  }
</script>

<div class="search" class:open={open && count > 0}>
  <Icon name="search" size={22} />
  <input
    bind:this={input}
    bind:value={query}
    placeholder="What do you want to play?"
    spellcheck="false"
    oninput={onInput}
    onkeydown={onKey}
    onfocus={() => (open = true)}
    onblur={() => setTimeout(() => (open = false), 150)}
  />
  {#if query}
    <button class="clear" title="Clear" onclick={() => { query = ""; suggestions = { queries: [], items: [] }; input?.focus(); }}><Icon name="close" size={16} /></button>
  {/if}
  {#if open && count > 0}
    <div class="dropdown" transition:fade={{ duration: 120 }}>
      {#each suggestions.queries as q, i (i)}
        <button class="query" class:active={active === i} onmousedown={(e) => e.preventDefault()} onclick={() => search(text(q))}>
          <Icon name="search" size={16} />
          <span>{#each q as r, k (k)}{#if k === 0}{r.text}{:else}<b>{r.text}</b>{/if}{/each}</span>
        </button>
      {/each}
      {#if suggestions.items.length}<div class="divider"></div>{/if}
      {#each suggestions.items as item, i (i)}
        <button class="item" class:active={active === suggestions.queries.length + i} onmousedown={(e) => e.preventDefault()} onclick={() => pick(item)}>
          <span class="thumb" class:circle={item.aspect === "circle"}>{#if art(item.thumbnails, 40)}<img src={art(item.thumbnails, 40)} alt="" />{/if}</span>
          <span class="meta"><span class="title">{item.title}</span><span class="sub"><Runs runs={item.subtitle} /></span></span>
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .search {
    position: relative; width: min(480px, 100%);
    display: flex; align-items: center; gap: 12px;
    height: 48px; padding: 0 10px 0 16px; border-radius: 999px;
    background: var(--bg-elevated); color: var(--text-muted);
    box-shadow: inset 0 0 0 1px transparent;
    transition: background var(--dur) var(--ease), box-shadow var(--dur) var(--ease), color var(--dur) var(--ease);
  }
  .search:hover { background: var(--bg-highlight); box-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.1); color: var(--text); }
  .search:focus-within { background: var(--bg-highlight); box-shadow: inset 0 0 0 2px var(--text); color: var(--text); }
  input { flex: 1; min-width: 0; height: 100%; border: 0; outline: 0; background: none; color: var(--text); font-size: 15px; }
  input::placeholder { color: var(--text-muted); }
  .clear {
    width: 30px; height: 30px; border-radius: 50%; display: grid; place-items: center; color: var(--text-muted);
    transition: color var(--dur-fast) var(--ease), transform var(--dur-fast) var(--ease);
  }
  .clear:hover { color: var(--text); transform: scale(1.1); }
  .dropdown {
    position: absolute; left: 0; right: 0; top: calc(100% + 8px); z-index: 50;
    padding: 6px; border-radius: var(--radius);
    background: var(--bg-highlight);
    box-shadow: var(--shadow-pop);
    max-height: 70vh; overflow-y: auto;
    transform-origin: top center;
    animation: drop var(--dur) var(--ease-out);
  }
  @keyframes drop { from { opacity: 0; transform: translateY(-6px) scale(0.98); } }
  .dropdown button { width: 100%; display: flex; align-items: center; gap: 12px; padding: 8px 10px; border-radius: 4px; text-align: left; color: var(--text-muted); transition: background var(--dur-fast) var(--ease); }
  .dropdown button:hover, .dropdown button.active { background: rgba(255, 255, 255, 0.1); color: var(--text); }
  .query span { color: var(--text-muted); }
  .query b { color: var(--text); font-weight: 700; }
  .divider { height: 1px; margin: 6px 4px; background: var(--border); }
  .thumb { width: 40px; height: 40px; flex: none; border-radius: 4px; overflow: hidden; background: var(--bg-elevated); }
  .thumb.circle { border-radius: 50%; }
  .thumb img { width: 100%; height: 100%; object-fit: cover; }
  .meta { min-width: 0; display: grid; }
  .title { color: var(--text); font-weight: 500; font-size: 14.5px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .sub { font-size: 13px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
</style>
