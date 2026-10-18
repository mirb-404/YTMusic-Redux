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
  let search = $state<ReturnType<typeof SearchBox>>();
  const entry = $derived(router.current);

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
  <div class="history" data-tauri-drag-region={drag}>
    <button class="round" class:video={videoOpen} title={videoOpen ? "Back to the pages (Esc)" : "Back (Alt+Left)"} disabled={!videoOpen && !router.canBack} onclick={back}><Icon name={videoOpen ? "chevronDown" : "chevronLeft"} size={22} /></button>
    <button class="round" title="Forward (Alt+Right)" disabled={!router.canForward} onclick={() => router.forward()}><Icon name="chevronRight" size={22} /></button>
  </div>
  <div class="center" data-tauri-drag-region={drag}><SearchBox bind:this={search} /></div>
  <div class="right">{@render controls()}</div>
</header>

<div class="body">
  <Sidebar {session} />
  <main class="content" bind:this={scroller}>
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
</div>

<QueuePanel {player} />
<Toast />
<ContextMenu />
<PlaylistDialog onchange={() => ui.playlists++} />

<style>
  .topbar {
    height: 64px; flex: none;
    display: grid; grid-template-columns: 240px minmax(0, 1fr) auto; align-items: center;
    padding-left: 12px;
    border-bottom: 1px solid var(--border);
    background: var(--bg);
  }
  .history { display: flex; gap: 6px; padding-left: 4px; }
  .round {
    width: 36px; height: 36px; border-radius: 50%;
    display: grid; place-items: center;
    background: var(--surface-hover); color: var(--text);
    transition: background 0.15s, opacity 0.15s;
  }
  .round:hover:not(:disabled) { background: var(--surface-active); }
  .round:disabled { opacity: 0.35; cursor: default; }
  .round.video { background: #fff; color: #000; }
  .center { display: flex; justify-content: center; padding: 0 24px; min-width: 0; }
  .right { display: flex; align-items: center; align-self: stretch; }

  .body { flex: 1; min-height: 0; display: flex; }
  .content { flex: 1; min-width: 0; overflow-y: auto; overflow-x: hidden; scrollbar-gutter: stable; }
  .content::-webkit-scrollbar { width: 12px; }
  .content::-webkit-scrollbar-thumb { background: rgba(255, 255, 255, 0.14); border-radius: 10px; border: 3px solid transparent; background-clip: padding-box; }
  .page { padding: 32px 40px 48px; animation: enter 0.32s var(--ease) both; }
  @keyframes enter { from { opacity: 0; transform: translateY(6px); } }

  .error { display: grid; justify-items: center; gap: 12px; padding: 80px 0; color: var(--text-muted); text-align: center; }
  .error button { display: inline-flex; align-items: center; gap: 8px; height: 36px; padding: 0 16px; border-radius: 18px; background: var(--surface-hover); color: var(--text); font-weight: 600; }

  .skeleton { display: grid; gap: 18px; }
  .sk { display: block; border-radius: 8px; background: linear-gradient(90deg, rgba(255,255,255,0.04), rgba(255,255,255,0.08), rgba(255,255,255,0.04)); background-size: 200% 100%; animation: shimmer 1.3s linear infinite; }
  .sk.title { width: 280px; height: 36px; margin-bottom: 12px; }
  .sk.heading { width: 220px; height: 24px; margin-top: 12px; }
  .sk-row { display: flex; gap: 20px; overflow: hidden; }
  .sk.card { width: 180px; aspect-ratio: 1; flex: none; }
  @keyframes shimmer { from { background-position: 100% 0; } to { background-position: -100% 0; } }
</style>
