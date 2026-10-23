<script lang="ts">
  import { listen } from "@tauri-apps/api/event";
  import { onMount, tick, type Snippet } from "svelte";
  import Icon from "../Icon.svelte";
  import ContextMenu from "./ContextMenu.svelte";
  import PageView from "./PageView.svelte";
  import PlaylistDialog from "./PlaylistDialog.svelte";
  import QueuePanel from "./QueuePanel.svelte";
  import Toast from "./Toast.svelte";
  import { api } from "../../lib/api";
  import type { PlayerState } from "../../lib/api";
  import SearchBox from "./SearchBox.svelte";
  import Sidebar from "./Sidebar.svelte";
  import { EXPLORE, HOME, LIBRARY, router, ui } from "../../lib/music.svelte";

  // The app's own YouTube Music pages: top bar with history and search, sidebar, and the
  // page, drawn from YTM's API instead of YTM's website.
  let {
    controls,
    drag,
    session,
    player,
    videoOpen = false
  }: { controls: Snippet; drag?: boolean; session: number; player: PlayerState | null; videoOpen?: boolean } = $props();

  // The video view (YTM's player page) covers the pages below the top bar: back leaves it,
  // and going anywhere else closes it.
  const closeVideo = () => api.command("togglePlayerPage");
  function back() {
    if (videoOpen) closeVideo();
    else router.back();
  }
  let lastIndex = router.index;
  $effect(() => {
    const index = router.index;
    if (index !== lastIndex && videoOpen) closeVideo();
    lastIndex = index;
  });

  let scroller = $state<HTMLElement>();
  let scrolled = $state(false);
  let search = $state<ReturnType<typeof SearchBox>>();
  const entry = $derived(router.current);
  const onHome = $derived(entry?.route.kind === "browse" && entry.route.endpoint.browseEndpoint?.browseId === HOME.browseEndpoint?.browseId);

  onMount(() => {
    if (!router.current) router.go({ kind: "browse", endpoint: HOME });
    // The other main pages load in the background, so the first visit is instant too.
    setTimeout(() => {
      router.prefetch({ kind: "browse", endpoint: EXPLORE });
      router.prefetch({ kind: "browse", endpoint: LIBRARY });
    }, 1500);
    const unlisten = listen<string>("browse-action", (e) => {
      const action = e.payload;
      if (action === "search") search?.focus();
      else if (action === "home") router.go({ kind: "browse", endpoint: HOME });
      else if (action === "explore") router.go({ kind: "browse", endpoint: EXPLORE });
      else if (action === "library") router.go({ kind: "browse", endpoint: LIBRARY });
      else if (action === "back") back();
      else if (action === "forward") router.forward();
      else if (action === "queue") ui.queueOpen = !ui.queueOpen;
    });
    // Mouse back and forward buttons.
    const onMouse = (e: MouseEvent) => {
      if (e.button === 3) router.back();
      else if (e.button === 4) router.forward();
    };
    window.addEventListener("mouseup", onMouse);
    return () => {
      unlisten.then((f) => f());
      window.removeEventListener("mouseup", onMouse);
    };
  });

  $effect(() => {
    router.scroller = scroller ?? null;
  });

  // A new page starts at the top; a page returned to restores where it was left.
  $effect(() => {
    const e = entry;
    if (!e?.page || !scroller) return;
    const top = e.scroll;
    tick().then(() => scroller?.scrollTo({ top }));
  });

  // Signing in changes every page; reload the one on screen.
  let firstSession = true;
  $effect(() => {
    session;
    if (firstSession) {
      firstSession = false;
      return;
    }
    router.reload();
  });
</script>

<header class="topbar" data-tauri-drag-region={drag}>
  <div class="history" data-tour="history" data-tauri-drag-region={drag}>
    <button class="round" class:video={videoOpen} title={videoOpen ? "Back to the pages (Esc)" : "Back (Alt+Left)"} disabled={!videoOpen && !router.canBack} onclick={back}><Icon name={videoOpen ? "chevronDown" : "chevronLeft"} size={22} /></button>
    <button class="round" title="Forward (Alt+Right)" disabled={!router.canForward} onclick={() => router.forward()}><Icon name="chevronRight" size={22} /></button>
  </div>
  <div class="center" data-tauri-drag-region={drag}>
    <button class="home" class:active={onHome} title="Home" onclick={() => router.go({ kind: "browse", endpoint: HOME })}><Icon name={onHome ? "homeFilled" : "home"} size={24} /></button>
    <div class="search-slot" data-tour="search"><SearchBox bind:this={search} /></div>
  </div>
  <div class="right">{@render controls()}</div>
</header>

<div class="body">
  <Sidebar {session} />
  <main class="content" class:behind={videoOpen} class:scrolled bind:this={scroller} onscroll={() => (scrolled = (scroller?.scrollTop ?? 0) > 4)}>
    {#if entry}
      {#key router.index}
        <div class="page">
          {#if entry.page}
            <PageView page={entry.page} />
          {:else if entry.error}
            <div class="error">
              <Icon name="warning" size={28} />
              <p>{entry.error}</p>
              <button onclick={() => router.reload()}><Icon name="refresh" size={16} />Try again</button>
            </div>
          {:else}
            <div class="skeleton" aria-label="Loading">
              <span class="sk title"></span>
              {#each [0, 1] as r (r)}
                <span class="sk heading"></span>
                <div class="sk-row">{#each [0, 1, 2, 3, 4, 5] as c (c)}<span class="sk card"></span>{/each}</div>
              {/each}
            </div>
          {/if}
        </div>
      {/key}
    {/if}
  </main>
  <QueuePanel {player} />
</div>

<Toast />
<ContextMenu />
<PlaylistDialog onchange={() => ui.playlists++} />

<style>
  .topbar {
    height: 64px; flex: none;
    display: grid; grid-template-columns: 280px minmax(0, 1fr) auto; align-items: center;
    /* A gutter above and below the 48px controls, the same 8px that separates the panels. */
    padding: var(--gutter) 0 var(--gutter) var(--gutter);
    background: var(--bg);
  }
  .history { display: flex; gap: 8px; padding-left: 8px; }
  .round {
    width: 32px; height: 32px; border-radius: 50%;
    display: grid; place-items: center;
    background: rgba(0, 0, 0, 0.7); color: var(--text);
    border: 1px solid var(--border);
    transition: transform var(--dur-fast) var(--ease), background var(--dur-fast) var(--ease), opacity var(--dur) var(--ease);
  }
  .round:hover:not(:disabled) { transform: scale(1.06); background: var(--bg-elevated); }
  .round:active:not(:disabled) { transform: scale(0.94); }
  .round:disabled { opacity: 0.4; cursor: default; }
  .round.video { background: #fff; color: #000; border-color: transparent; }
  .center { display: flex; justify-content: center; align-items: center; gap: 8px; padding: 0 24px; min-width: 0; }
  .home {
    width: 48px; height: 48px; flex: none; border-radius: 50%;
    display: grid; place-items: center;
    background: var(--bg-elevated); color: var(--text-muted);
    transition: transform var(--dur-fast) var(--ease), background var(--dur-fast) var(--ease), color var(--dur-fast) var(--ease);
  }
  .home:hover { transform: scale(1.04); background: var(--bg-highlight); color: var(--text); }
  .home:active { transform: scale(0.96); }
  .home.active { color: var(--text); }
  .search-slot { width: min(480px, 100%); display: flex; }
  .right { display: flex; align-items: center; align-self: start; height: 48px; margin-top: calc(-1 * var(--gutter)); }

  /* Spotify's layout: rounded panels floating on black, a gutter apart. */
  .body { flex: 1; min-height: 0; display: flex; gap: var(--gutter); padding: var(--gutter); padding-top: 0; }
  .content {
    position: relative;
    flex: 1; min-width: 0; overflow-y: auto; overflow-x: hidden; scrollbar-gutter: stable;
    border-radius: var(--radius-panel);
    /* YouTube Music's red bleeding into Spotify's panel gray at the top of every page. */
    background:
      linear-gradient(180deg, color-mix(in srgb, var(--accent) 16%, transparent) 0, transparent 260px),
      var(--bg-panel);
    transform-origin: 50% 30%;
    transition: transform var(--dur-slow) cubic-bezier(0.65, 0, 0.35, 1), opacity var(--dur-slow) cubic-bezier(0.65, 0, 0.35, 1);
  }
  .content::before {
    content: ""; position: sticky; top: 0; z-index: 5; display: block; height: 0;
    box-shadow: 0 0 0 0 transparent;
    transition: box-shadow var(--dur) var(--ease);
  }
  .content.scrolled::before { box-shadow: 0 0 18px 8px rgba(0, 0, 0, 0.6); }
  /* Recedes under the video view and comes forward as it slides away. */
  .content.behind { transform: scale(0.97); opacity: 0.35; }
  .content::-webkit-scrollbar { width: 12px; }
  .content::-webkit-scrollbar-thumb { background: rgba(255, 255, 255, 0.2); border-radius: 0; border: 3px solid transparent; background-clip: padding-box; }
  .content::-webkit-scrollbar-thumb:hover { background-color: rgba(255, 255, 255, 0.34); }
  .page { padding: 24px 32px 48px; animation: enter var(--dur-slow) var(--ease-out) both; }
  @keyframes enter { from { opacity: 0; transform: translateY(10px); } }

  .error { display: grid; justify-items: center; gap: 12px; padding: 80px 0; color: var(--text-muted); text-align: center; }
  .error button {
    display: inline-flex; align-items: center; gap: 8px; height: 40px; padding: 0 22px; border-radius: 999px;
    background: #fff; color: #000; font-weight: 700;
    transition: transform var(--dur-fast) var(--ease);
  }
  .error button:hover { transform: scale(1.04); }

  .skeleton { display: grid; gap: 18px; }
  .sk { display: block; border-radius: var(--radius); background: linear-gradient(90deg, rgba(255,255,255,0.05), rgba(255,255,255,0.1), rgba(255,255,255,0.05)); background-size: 200% 100%; animation: shimmer 1.3s linear infinite; }
  .sk.title { width: 280px; height: 36px; margin-bottom: 12px; }
  .sk.heading { width: 220px; height: 24px; margin-top: 12px; }
  .sk-row { display: flex; gap: 12px; overflow: hidden; }
  .sk.card { width: 180px; height: 240px; flex: none; }
  @keyframes shimmer { from { background-position: 100% 0; } to { background-position: -100% 0; } }
</style>
