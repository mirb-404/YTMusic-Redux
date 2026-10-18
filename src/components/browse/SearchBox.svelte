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
  <Icon name="search" size={20} />
  <input
    bind:this={input}
    bind:value={query}
    placeholder="Search songs, albums, artists, podcasts"
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
    position: relative; width: min(520px, 100%);
    display: flex; align-items: center; gap: 10px;
    height: 40px; padding: 0 8px 0 14px; border-radius: 20px;
    background: var(--surface-hover); border: 1px solid transparent; color: var(--text-muted);
    transition: background 0.15s, border-color 0.15s, border-radius 0.15s;
  }
  .search:focus-within { background: var(--bg-elevated); border-color: var(--border); color: var(--text); }
  .search.open { border-radius: 20px 20px 0 0; }
  input { flex: 1; min-width: 0; height: 100%; border: 0; outline: 0; background: none; color: var(--text); font-size: 14px; }
  input::placeholder { color: var(--text-faint); }
  .clear { width: 28px; height: 28px; border-radius: 50%; display: grid; place-items: center; color: var(--text-muted); }
  .clear:hover { background: var(--surface-hover); color: var(--text); }
  .dropdown {
    position: absolute; left: -1px; right: -1px; top: 100%; z-index: 50;
    padding: 6px; border-radius: 0 0 16px 16px;
    background: var(--bg-elevated); border: 1px solid var(--border); border-top: 0;
    box-shadow: 0 24px 48px rgba(0, 0, 0, 0.55);
    max-height: 70vh; overflow-y: auto;
  }
  .dropdown button { width: 100%; display: flex; align-items: center; gap: 12px; padding: 8px 10px; border-radius: 8px; text-align: left; color: var(--text-muted); }
  .dropdown button:hover, .dropdown button.active { background: var(--surface-hover); color: var(--text); }
  .query span { color: var(--text-muted); }
  .query b { color: var(--text); font-weight: 700; }
  .divider { height: 1px; margin: 6px 4px; background: var(--border); }
  .thumb { width: 40px; height: 40px; flex: none; border-radius: 6px; overflow: hidden; background: var(--surface-hover); }
  .thumb.circle { border-radius: 50%; }
  .thumb img { width: 100%; height: 100%; object-fit: cover; }
  .meta { min-width: 0; display: grid; }
  .title { color: var(--text); font-weight: 600; font-size: 13.5px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .sub { font-size: 12.5px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
</style>
