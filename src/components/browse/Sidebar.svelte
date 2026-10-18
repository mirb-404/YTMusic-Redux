<script lang="ts">
  import { onMount } from "svelte";
  import Icon from "../Icon.svelte";
  import type { IconName } from "../../lib/icons";
  import { EXPLORE, HISTORY, HOME, LIBRARY, music, router, ui, type Endpoint, type GuideEntry } from "../../lib/music.svelte";

  // Refreshed when the shell sees the session change (signing in).
  let { session }: { session: number } = $props();

  const nav: { label: string; icon: IconName; endpoint: Endpoint }[] = [
    { label: "Home", icon: "home", endpoint: HOME },
    { label: "Explore", icon: "explore", endpoint: EXPLORE },
    { label: "Library", icon: "library", endpoint: LIBRARY },
    { label: "History", icon: "history", endpoint: HISTORY }
  ];

  let playlists = $state<GuideEntry[]>([]);
  let signedIn = $state(true);

  const activeId = $derived(router.current?.route.kind === "browse" ? router.current.route.endpoint.browseEndpoint?.browseId : null);

  async function refresh() {
    signedIn = await music.signedIn().catch(() => true);
    playlists = signedIn ? await music.guide().catch(() => []) : [];
  }

  onMount(() => {
    const timer = setInterval(refresh, 5 * 60_000);
    return () => clearInterval(timer);
  });

  $effect(() => {
    session;
    ui.playlists;
    refresh();
  });
</script>

<aside class="sidebar">
  <nav class="main" data-tour="sidebar">
    {#each nav as n (n.label)}
      <button class:active={activeId === n.endpoint.browseEndpoint?.browseId} onclick={() => router.go({ kind: "browse", endpoint: n.endpoint })}>
        <Icon name={n.icon} size={22} /><span>{n.label}</span>
      </button>
    {/each}
  </nav>

  {#if !signedIn}
    <div class="signin">
      <p>Sign in for your library, playlists and recommendations.</p>
      <button onclick={() => music.signIn()}><Icon name="account" size={18} />Sign in</button>
    </div>
  {:else}
    <div class="label-row">
      <span class="label">Playlists</span>
      <button class="new" title="New playlist" onclick={() => (ui.dialog = { target: null, create: true })}><Icon name="add" size={18} /></button>
    </div>
    <div class="playlists">
      {#each playlists as p (p.endpoint.browseEndpoint?.browseId)}
        <div class="playlist" class:active={activeId === p.endpoint.browseEndpoint?.browseId}>
          <button class="open" onclick={() => router.go({ kind: "browse", endpoint: p.endpoint })}>
            <span class="name">{p.title}</span>
            {#if p.subtitle}<span class="owner">{p.subtitle}</span>{/if}
          </button>
          {#if p.play}<button class="play" title="Play {p.title}" onclick={() => music.play(p.play!)}><Icon name="play" size={16} /></button>{/if}
        </div>
      {/each}
    </div>
  {/if}
</aside>

<style>
  .sidebar { width: 240px; flex: none; display: flex; flex-direction: column; min-height: 0; padding: 12px 10px 12px 12px; border-right: 1px solid var(--border); }
  .main { display: grid; gap: 2px; }
  .main button {
    display: flex; align-items: center; gap: 14px;
    height: 44px; padding: 0 14px; border-radius: 10px;
    font-weight: 600; color: var(--text-muted);
    transition: background 0.15s, color 0.15s;
  }
  .main button:hover { background: var(--surface); color: var(--text); }
  .main button.active { background: var(--surface-hover); color: var(--text); }
  .main button.active :global(svg) { color: var(--accent); }
  .label-row { display: flex; align-items: center; justify-content: space-between; margin: 22px 6px 8px 14px; }
  .new { width: 28px; height: 28px; border-radius: 50%; display: grid; place-items: center; color: var(--text-muted); }
  .new:hover { background: var(--surface-hover); color: var(--text); }
  .label { font-size: 11.5px; font-weight: 700; letter-spacing: 0.8px; text-transform: uppercase; color: var(--text-faint); }
  .playlists { overflow-y: auto; min-height: 0; scrollbar-width: thin; padding-bottom: 8px; }
  .playlist { position: relative; display: flex; align-items: center; border-radius: 8px; transition: background 0.15s; }
  .playlist:hover { background: var(--surface); }
  .playlist.active { background: var(--surface-hover); }
  .open { flex: 1; min-width: 0; display: grid; text-align: left; padding: 7px 14px; }
  .name { font-size: 13.5px; font-weight: 600; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .owner { font-size: 12px; color: var(--text-faint); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .play {
    width: 30px; height: 30px; margin-right: 6px; border-radius: 50%; flex: none;
    display: grid; place-items: center; background: #fff; color: #000;
    opacity: 0; transform: scale(0.85); transition: opacity 0.15s, transform 0.15s;
  }
  .playlist:hover .play { opacity: 1; transform: none; }
  .signin { margin: 22px 8px 0; padding: 16px; border-radius: 12px; background: var(--surface); border: 1px solid var(--border); }
  .signin p { margin: 0 0 12px; font-size: 13px; color: var(--text-muted); line-height: 1.5; }
  .signin button { display: inline-flex; align-items: center; gap: 8px; height: 36px; padding: 0 16px; border-radius: 18px; background: #fff; color: #000; font-weight: 700; }
</style>
