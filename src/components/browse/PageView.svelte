<script lang="ts">
  import Icon from "../Icon.svelte";
  import Card from "./Card.svelte";
  import PageHeader from "./PageHeader.svelte";
  import Runs from "./Runs.svelte";
  import Shelf from "./Shelf.svelte";
  import TrackRow from "./TrackRow.svelte";
  import { art, music, router, type Page, type Section } from "../../lib/music.svelte";

  let { page }: { page: Page } = $props();

  // Album track lists number their rows instead of repeating the cover.
  const numbered = $derived(page.header?.kind === "collection" && page.sections[0]?.items.every((i) => i.index));
  // The last shelf or the page itself continues as it scrolls into view.
  const lastList = $derived([...page.sections].reverse().find((s) => s.kind === "list" && s.continuation) ?? null);

  function sentinel(node: HTMLElement, target: Section | null) {
    let current = target;
    const observer = new IntersectionObserver(
      (entries) => {
        if (entries.some((e) => e.isIntersecting)) router.more(current ?? undefined);
      },
      { root: router.scroller, rootMargin: "800px 0px" }
    );
    observer.observe(node);
    return {
      update(next: Section | null) {
        current = next;
      },
      destroy: () => observer.disconnect()
    };
  }

  const searchQuery = $derived(router.current?.route.kind === "search" ? router.current.route.query : null);
</script>

{#if page.header}<PageHeader header={page.header} />{/if}

{#if page.chips.length}
  <nav class="chips">
    {#each page.chips as chip (chip.text)}
      <button
        class:selected={chip.selected}
        onclick={() => {
          const e = chip.endpoint;
          if (e?.searchEndpoint && searchQuery !== null) router.go({ kind: "search", query: searchQuery, params: chip.selected ? undefined : e.searchEndpoint.params });
          else router.open(e);
        }}>{chip.text}</button
      >
    {/each}
  </nav>
{/if}

{#each page.sections as section, i (i)}
  <div class="section" style:--delay="{Math.min(i, 5) * 40}ms">
    {#if section.kind === "carousel"}
      <Shelf {section} />
    {:else if section.kind === "card"}
      {@const top = section.items[0]}
      <section class="top">
        <h2>{section.title || "Top result"}</h2>
        <div class="top-grid">
          <div class="top-card" role="button" tabindex="0" onclick={() => router.open(top.endpoint)} onkeydown={(e) => e.key === "Enter" && router.open(top.endpoint)}>
            <div class="top-art" class:circle={top.aspect === "circle"}>{#if art(top.thumbnails, 120)}<img src={art(top.thumbnails, 120)} alt="" />{/if}</div>
            <div class="top-title">{top.title}</div>
            <div class="top-sub"><Runs runs={top.subtitle} /></div>
            {#if top.play ?? (top.endpoint?.watchEndpoint ? top.endpoint : null)}
              <button
                class="top-play"
                title="Play"
                onclick={(e) => {
                  e.stopPropagation();
                  music.play((top.play ?? top.endpoint)!);
                }}><Icon name="play" size={24} /></button
              >
            {/if}
          </div>
          {#if section.items.length > 1}
            <div class="top-list">{#each section.items.slice(1) as item, j (j)}<TrackRow {item} />{/each}</div>
          {/if}
        </div>
      </section>
    {:else if section.kind === "list"}
      <section class="list">
        {#if section.title}
          <header>
            <h2>{section.title}</h2>
            {#if section.more}<button class="more" onclick={() => router.open(section.more)}>Show all</button>{/if}
          </header>
        {/if}
        {#each section.items as item, j (j)}<TrackRow {item} numbered={numbered && i === 0} />{/each}
      </section>
    {:else if section.kind === "grid"}
      <section class="grid-section">
        {#if section.title}<h2>{section.title}</h2>{/if}
        <div class="grid">{#each section.items as item, j (j)}<Card {item} width={176} />{/each}</div>
      </section>
    {:else if section.kind === "description"}
      <section class="about">
        <h2>{section.title || "About"}</h2>
        <p>{section.text}</p>
      </section>
    {/if}
  </div>
{/each}

{#if lastList}
  <div class="sentinel" use:sentinel={lastList}></div>
{:else if page.continuation}
  <div class="sentinel" use:sentinel={null}></div>
{/if}
{#if router.current?.loadingMore}<div class="spinner" aria-label="Loading"></div>{/if}

<style>
  .section { animation: rise var(--dur-slow) var(--ease-out) both; animation-delay: var(--delay); }
  @keyframes rise { from { opacity: 0; transform: translateY(14px); } }
  h2 { margin: 0 0 12px; font-size: 24px; font-weight: 800; letter-spacing: -0.02em; }

  .chips { display: flex; flex-wrap: wrap; gap: 8px; margin: 0 0 24px; }
  .chips button {
    height: 32px; padding: 0 14px; border-radius: 999px;
    font-size: 14px; font-weight: 500;
    background: var(--surface-hover); color: var(--text);
    transition: background var(--dur-fast) var(--ease), color var(--dur-fast) var(--ease), transform var(--dur-fast) var(--ease);
  }
  .chips button:hover { background: var(--surface-active); }
  .chips button:active { transform: scale(0.96); }
  .chips button.selected { background: #fff; color: #000; }

  .list { margin-bottom: 36px; }
  .list header { display: flex; align-items: center; justify-content: space-between; }
  .more {
    margin-bottom: 12px; padding: 0 4px;
    font-size: 14px; font-weight: 700; color: var(--text-muted);
    transition: color var(--dur-fast) var(--ease);
  }
  .more:hover { color: var(--text); text-decoration: underline; }

  .grid-section { margin-bottom: 36px; }
  .grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(180px, 1fr)); gap: 24px 0; margin: 0 -12px; padding-top: 12px; }
  .grid :global(.card) { width: auto; }

  .top { margin-bottom: 36px; }
  .top-grid { display: grid; grid-template-columns: minmax(280px, 420px) minmax(0, 1fr); gap: 24px; align-items: start; }
  .top-card {
    position: relative; padding: 20px; border-radius: var(--radius); cursor: pointer;
    background: var(--surface);
    transition: background var(--dur-slow) var(--ease);
  }
  .top-card:hover { background: var(--surface-hover); }
  .top-art { width: 92px; height: 92px; border-radius: var(--radius-sm); overflow: hidden; background: var(--bg-highlight); box-shadow: var(--shadow-card); }
  .top-art.circle { border-radius: 50%; }
  .top-art img { width: 100%; height: 100%; object-fit: cover; }
  .top-title { margin-top: 20px; font-size: 32px; font-weight: 800; letter-spacing: -0.03em; line-height: 1.15; }
  .top-sub { margin-top: 4px; font-size: 14px; color: var(--text-muted); }
  .top-play {
    position: absolute; right: 20px; bottom: 20px; width: 56px; height: 56px; border-radius: 50%;
    display: grid; place-items: center; background: var(--accent); color: var(--accent-ink);
    box-shadow: 0 8px 8px rgba(0, 0, 0, 0.3);
    opacity: 0; transform: translateY(8px);
    transition: opacity var(--dur-slow) var(--ease), transform var(--dur-slow) var(--ease), background var(--dur-fast) var(--ease);
  }
  .top-card:hover .top-play { opacity: 1; transform: none; }
  .top-play:hover { transform: scale(1.06) !important; background: var(--accent-hover); }
  @media (max-width: 980px) { .top-grid { grid-template-columns: 1fr; } }

  .about { margin-bottom: 36px; max-width: 820px; padding: 24px; border-radius: var(--radius); background: var(--surface); }
  .about p { margin: 0; color: var(--text-muted); line-height: 1.65; white-space: pre-line; }

  .sentinel { height: 1px; }
  .spinner {
    width: 28px; height: 28px; margin: 12px auto 40px; border-radius: 50%;
    border: 3px solid var(--surface-active); border-top-color: var(--accent); animation: spin 0.8s linear infinite;
  }
  @keyframes spin { to { transform: rotate(360deg); } }
</style>
