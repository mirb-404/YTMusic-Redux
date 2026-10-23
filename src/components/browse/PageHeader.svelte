<script lang="ts">
  import Icon from "../Icon.svelte";
  import Runs from "./Runs.svelte";
  import { art, music, openMenu, runEntry, type Header } from "../../lib/music.svelte";

  let { header }: { header: Header } = $props();

  const cover = $derived(art(header.thumbnails, header.kind === "artist" ? 1200 : 260));
  let expanded = $state(false);
</script>

{#if header.kind === "plain"}
  <h1 class="plain">{header.title}</h1>
{:else}
  <header class="hero" class:artist={header.kind === "artist"}>
    {#if cover}
      {#if header.kind === "artist"}
        <img class="banner" src={cover} alt="" />
      {:else}
        <img class="glow" src={cover} alt="" aria-hidden="true" />
      {/if}
    {/if}
    <div class="content">
      {#if header.kind === "collection"}
        <div class="cover">{#if cover}<img src={cover} alt="" />{/if}</div>
      {/if}
      <div class="info">
        {#if header.kind === "artist"}<span class="kind">Artist</span>{/if}
        <h1>{header.title}</h1>
        {#if header.subtitle.length}<div class="subtitle"><Runs runs={header.subtitle} /></div>{/if}
        {#if header.secondSubtitle.length}<div class="second"><Runs runs={header.secondSubtitle} /></div>{/if}
        {#if header.description}
          <button class="description" class:expanded onclick={() => (expanded = !expanded)}>{header.description}</button>
        {/if}
      </div>
    </div>
  </header>
  <!-- Spotify's action row: a big play button over the header color fading into the page. -->
  <div class="action-bar">
    {#if header.play}
      <button class="play-big" title="Play" onclick={() => music.play(header.play!)}><Icon name="play" size={28} /></button>
    {:else if header.shuffle}
      <button class="play-big" title="Shuffle" onclick={() => music.play(header.shuffle!)}><Icon name="shuffle" size={26} /></button>
    {/if}
    {#if header.shuffle && header.play}
      <button class="ghost" title="Shuffle" onclick={() => music.play(header.shuffle!)}><Icon name="shuffle" size={28} /></button>
    {/if}
    {#if header.radio}
      <button class="pill" onclick={() => music.play(header.radio!)}><Icon name="radio" size={18} />Radio</button>
    {/if}
    {#if header.save}
      <button class="ghost" class:saved={header.save.icon === "BOOKMARK"} title={header.save.text} onclick={() => runEntry(header.save!, null, header.title, header.menu)}>
        <Icon name={header.save.icon === "BOOKMARK" ? "bookmark" : "bookmarkBorder"} size={28} />
      </button>
    {/if}
    {#if header.menu.length}
      <button class="ghost" title="More" onclick={(e) => openMenu(e, header.menu, null, header.title)}><Icon name="more" size={28} /></button>
    {/if}
  </div>
{/if}

<style>
  .plain { margin: 8px 0 24px; font-size: 32px; font-weight: 800; letter-spacing: -0.03em; }
  .hero { position: relative; margin: -24px -32px 0; padding: 64px 32px 24px; isolation: isolate; overflow: hidden; }
  .glow {
    position: absolute; inset: -40%; z-index: -1; width: 180%; height: 180%;
    object-fit: cover; filter: blur(90px) saturate(1.8); opacity: 0.55;
    animation: glow-in 0.8s var(--ease) both;
  }
  @keyframes glow-in { from { opacity: 0; } }
  /* Darkens toward the page, the way Spotify's header color settles into its list. */
  .hero::after {
    content: ""; position: absolute; inset: 0; z-index: -1; pointer-events: none;
    background: linear-gradient(to bottom, transparent 0%, rgba(0, 0, 0, 0.45) 100%);
  }
  .artist { min-height: 380px; display: flex; align-items: flex-end; }
  .banner {
    position: absolute; inset: 0; z-index: -1; width: 100%; height: 100%; object-fit: cover; object-position: center 25%;
    animation: banner-in 1.2s var(--ease-out) both;
  }
  @keyframes banner-in { from { opacity: 0; transform: scale(1.06); } }
  /* Keeps the name and text readable over bright banners. */
  .artist::before {
    content: ""; position: absolute; inset: 0; z-index: -1;
    background: linear-gradient(to top, rgba(18, 18, 18, 0.95) 0%, rgba(0, 0, 0, 0.3) 55%, transparent 100%);
  }
  .artist::after { display: none; }
  .artist .subtitle, .artist .description { color: rgba(255, 255, 255, 0.85); }
  .content { display: flex; align-items: flex-end; gap: 24px; min-width: 0; }
  .cover {
    width: 232px; height: 232px; flex: none; border-radius: var(--radius-sm); overflow: hidden;
    background: var(--bg-highlight); box-shadow: 0 8px 40px rgba(0, 0, 0, 0.6);
    animation: cover-in 0.5s var(--ease-out) both;
  }
  @keyframes cover-in { from { opacity: 0; transform: scale(0.94); } }
  .cover img { width: 100%; height: 100%; object-fit: cover; }
  .info { min-width: 0; display: grid; gap: 6px; animation: info-in 0.5s var(--ease-out) 0.06s both; }
  @keyframes info-in { from { opacity: 0; transform: translateY(8px); } }
  .kind { font-size: 13px; font-weight: 600; }
  h1 { margin: 0 0 6px; font-size: clamp(36px, 5.4vw, 80px); font-weight: 900; line-height: 1; letter-spacing: -0.04em; }
  .artist h1 { font-size: clamp(48px, 7vw, 96px); text-shadow: 0 4px 30px rgba(0, 0, 0, 0.5); }
  .subtitle { font-size: 14px; font-weight: 600; color: var(--text); }
  .second { font-size: 13.5px; color: var(--text-muted); }
  .description {
    max-width: 720px; padding: 0; text-align: left; font-size: 14px; line-height: 1.55; color: var(--text-muted);
    display: -webkit-box; -webkit-line-clamp: 2; line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; cursor: pointer;
  }
  .description.expanded { -webkit-line-clamp: unset; line-clamp: unset; }

  .action-bar {
    position: relative; display: flex; align-items: center; gap: 24px;
    margin: 0 -32px 24px; padding: 24px 32px;
    background: linear-gradient(to bottom, rgba(0, 0, 0, 0.45), transparent);
  }
  .action-bar button { display: grid; place-items: center; transition: transform var(--dur-fast) var(--ease), color var(--dur-fast) var(--ease), background var(--dur-fast) var(--ease); }
  .play-big {
    width: 56px; height: 56px; border-radius: 50%;
    background: var(--accent); color: var(--accent-ink);
    box-shadow: 0 8px 24px color-mix(in srgb, var(--accent) 35%, transparent);
  }
  .play-big:hover { transform: scale(1.06); background: var(--accent-hover); }
  .play-big:active { transform: scale(0.96); }
  .ghost { width: 40px; height: 40px; color: var(--text-muted); }
  .ghost:hover { color: var(--text); transform: scale(1.06); }
  .ghost:active { transform: scale(0.94); }
  .ghost.saved { color: var(--accent); }
  .pill {
    display: inline-flex !important; align-items: center; gap: 8px;
    height: 34px; padding: 0 16px; border-radius: 999px;
    border: 1px solid rgba(255, 255, 255, 0.3); font-weight: 700; font-size: 13.5px;
  }
  .pill:hover { border-color: var(--text); transform: scale(1.04); }
</style>
