<script lang="ts">
  import { onMount } from "svelte";
  import Icon from "../Icon.svelte";
  import type { IconName } from "../../lib/icons";
  import { art, auth, EXPLORE, HISTORY, HOME, LIBRARY, music, refreshAccount, router, ui, type Endpoint, type GuideEntry } from "../../lib/music.svelte";

  // The page on screen shows its icon filled.
  const nav: { label: string; icon: IconName; active: IconName; endpoint: Endpoint }[] = [
    { label: "Home", icon: "home", active: "homeFilled", endpoint: HOME },
    { label: "Explore", icon: "explore", active: "exploreFilled", endpoint: EXPLORE },
    { label: "Library", icon: "library", active: "libraryFilled", endpoint: LIBRARY },
    { label: "History", icon: "history", active: "history", endpoint: HISTORY }
  ];

  let playlists = $state<GuideEntry[]>([]);
  // Unknown until the first check: neither the playlists nor the sign-in card show yet.
  const signedIn = $derived(auth.account !== null);

  const activeId = $derived(router.current?.route.kind === "browse" ? router.current.route.endpoint.browseEndpoint?.browseId : null);

  // A stable color per playlist, so its tile stays the same between visits.
  function hue(title: string) {
    let h = 0;
    for (const ch of title) h = (h * 31 + ch.charCodeAt(0)) | 0;
    return Math.abs(h) % 360;
  }

  async function refresh() {
    await refreshAccount();
    playlists = auth.account ? await music.guide().catch(() => []) : [];
  }

  onMount(() => {
    const timer = setInterval(refresh, 5 * 60_000);
    return () => clearInterval(timer);
  });

  // Reloaded when the session changes (signing in or out).
  $effect(() => {
    auth.session;
    ui.playlists;
    refresh();
  });
</script>

<aside class="sidebar">
  <nav class="main panel" data-tour="sidebar">
    {#each nav as n (n.label)}
      {@const active = activeId === n.endpoint.browseEndpoint?.browseId}
      <button class:active onclick={() => router.go({ kind: "browse", endpoint: n.endpoint })}>
        <Icon name={active ? n.active : n.icon} size={24} /><span>{n.label}</span>
      </button>
    {/each}
  </nav>

  <section class="library panel">
    <div class="label-row">
      <span class="label"><Icon name="library" size={24} />Your Library</span>
      {#if auth.account}
        <button class="new" title="New playlist" onclick={() => (ui.dialog = { target: null, create: true })}><Icon name="add" size={18} /></button>
      {/if}
    </div>
    {#if !signedIn}
      <div class="signin">
        <h3>Make it yours</h3>
        <p>Sign in for your library, playlists and recommendations.</p>
        <button onclick={() => music.signIn()}><Icon name="account" size={18} />Sign in</button>
      </div>
    {:else}
      <div class="playlists">
        {#each playlists as p, i (p.endpoint.browseEndpoint?.browseId)}
          <div class="playlist" class:active={activeId === p.endpoint.browseEndpoint?.browseId} style:--i={Math.min(i, 12)}>
            <button class="open" onclick={() => router.go({ kind: "browse", endpoint: p.endpoint })}>
              {#if p.thumbnails.length}
                <img class="tile" src={art(p.thumbnails, 48)} alt="" loading="lazy" decoding="async" />
              {:else}
                <span class="tile" style:--hue={hue(p.title)} aria-hidden="true">{p.title.trim().charAt(0).toUpperCase()}</span>
              {/if}
              <span class="text">
                <span class="name">{p.title}</span>
                <span class="owner">{p.subtitle.toLowerCase().includes("playlist") ? p.subtitle : ["Playlist", p.subtitle].filter(Boolean).join(" · ")}</span>
              </span>
            </button>
            {#if p.play}<button class="play" title="Play {p.title}" onclick={() => music.play(p.play!)}><Icon name="play" size={16} /></button>{/if}
          </div>
        {/each}
      </div>
    {/if}
  </section>
</aside>

<style>
  .sidebar { width: 280px; flex: none; display: flex; flex-direction: column; gap: var(--gutter); min-height: 0; }
  .panel { border-radius: var(--radius-panel); background: var(--bg-panel); }
  .main { display: grid; gap: 2px; padding: 8px; }
  .main button {
    display: flex; align-items: center; gap: 18px;
    height: 44px; padding: 0 8px; border-radius: var(--radius-sm);
    font-weight: 700; font-size: 15px; color: var(--text-muted);
    transition: color var(--dur) var(--ease);
  }
  .main button:hover { color: var(--text); }
  .main button.active { color: var(--text); }
  .main button :global(svg) { transition: color var(--dur) var(--ease), transform var(--dur) var(--ease-spring); }
  .main button:active :global(svg) { transform: scale(0.9); }
  .main button.active :global(svg) { color: var(--accent); }

  .library { flex: 1; min-height: 0; display: flex; flex-direction: column; padding: 4px 8px 8px; }
  .label-row { display: flex; align-items: center; justify-content: space-between; padding: 10px 0 10px 8px; }
  .label { display: flex; align-items: center; gap: 18px; font-size: 15px; font-weight: 700; color: var(--text-muted); }
  .new {
    width: 32px; height: 32px; border-radius: 50%; display: grid; place-items: center; color: var(--text-muted);
    transition: background var(--dur-fast) var(--ease), color var(--dur-fast) var(--ease), transform var(--dur-fast) var(--ease);
  }
  .new:hover { background: var(--bg-elevated); color: var(--text); transform: scale(1.06); }
  .new:active { transform: scale(0.94); }
  .playlists { overflow-y: auto; min-height: 0; padding-bottom: 8px; }
  .playlists::-webkit-scrollbar { width: 8px; }
  .playlist {
    position: relative; display: flex; align-items: center; border-radius: var(--radius-sm);
    transition: background var(--dur) var(--ease);
    animation: in var(--dur-slow) var(--ease-out) both; animation-delay: calc(var(--i) * 18ms);
  }
  @keyframes in { from { opacity: 0; transform: translateX(-6px); } }
  .playlist:hover { background: var(--surface-hover); }
  .playlist.active { background: var(--surface-active); }
  .open { flex: 1; min-width: 0; display: flex; align-items: center; gap: 12px; text-align: left; padding: 8px; }
  /* YTM's playlist cover; a playlist without one gets a tile in its own color. */
  .tile {
    width: 48px; height: 48px; flex: none; border-radius: 4px;
    display: grid; place-items: center;
    font-size: 20px; font-weight: 800; color: rgba(255, 255, 255, 0.92);
    background: linear-gradient(135deg, hsl(var(--hue) 70% 48%), hsl(calc(var(--hue) + 40) 75% 28%));
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.35);
  }
  img.tile { object-fit: cover; background: var(--bg-highlight); }
  .text { min-width: 0; display: grid; gap: 2px; }
  .name { font-size: 15px; font-weight: 500; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .active .name { color: var(--accent); }
  .owner { font-size: 13px; color: var(--text-muted); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .play {
    width: 32px; height: 32px; margin-right: 8px; border-radius: 50%; flex: none;
    display: grid; place-items: center; background: var(--accent); color: var(--accent-ink);
    box-shadow: 0 6px 14px rgba(0, 0, 0, 0.4);
    opacity: 0; transform: translateY(4px) scale(0.9);
    transition: opacity var(--dur) var(--ease), transform var(--dur) var(--ease), background var(--dur-fast) var(--ease);
  }
  .playlist:hover .play { opacity: 1; transform: none; }
  .play:hover { background: var(--accent-hover); transform: scale(1.08) !important; }
  .signin { margin: 8px 4px 0; padding: 18px 20px; border-radius: var(--radius); background: var(--bg-elevated); }
  .signin h3 { margin: 0 0 6px; font-size: 16px; }
  .signin p { margin: 0 0 16px; font-size: 13.5px; color: var(--text); line-height: 1.5; }
  .signin button {
    display: inline-flex; align-items: center; gap: 8px; height: 34px; padding: 0 16px; border-radius: 999px;
    background: #fff; color: #000; font-weight: 700; font-size: 13.5px;
    transition: transform var(--dur-fast) var(--ease);
  }
  .signin button:hover { transform: scale(1.04); }
  .signin button:active { transform: scale(0.97); }
  @media (max-width: 960px) { .sidebar { width: 240px; } }
</style>
