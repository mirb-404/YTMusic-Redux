<script lang="ts">
  import Icon from "../Icon.svelte";
  import Runs from "./Runs.svelte";
  import { art, music, type Header } from "../../lib/music.svelte";

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
        <h1>{header.title}</h1>
        {#if header.subtitle.length}<div class="subtitle"><Runs runs={header.subtitle} /></div>{/if}
        {#if header.secondSubtitle.length}<div class="second"><Runs runs={header.secondSubtitle} /></div>{/if}
        {#if header.description}
          <button class="description" class:expanded onclick={() => (expanded = !expanded)}>{header.description}</button>
        {/if}
        <div class="actions">
          {#if header.play}<button class="primary" onclick={() => music.play(header.play!)}><Icon name="play" size={20} />Play</button>{/if}
          {#if header.shuffle}<button class={header.play ? "secondary" : "primary"} onclick={() => music.play(header.shuffle!)}><Icon name="shuffle" size={18} />Shuffle</button>{/if}
          {#if header.radio}<button class="secondary" onclick={() => music.play(header.radio!)}><Icon name="radio" size={18} />Radio</button>{/if}
        </div>
      </div>
    </div>
  </header>
{/if}

<style>
  .plain { margin: 8px 0 28px; font-size: 34px; font-weight: 800; letter-spacing: -0.02em; }
  .hero { position: relative; margin: -32px -40px 36px; padding: 48px 40px 32px; isolation: isolate; overflow: hidden; }
  .glow {
    position: absolute; inset: -40%; z-index: -1; width: 180%; height: 180%;
    object-fit: cover; filter: blur(80px) saturate(1.6); opacity: 0.4;
    mask-image: linear-gradient(to bottom, #000 30%, transparent 85%);
  }
  .artist { min-height: 420px; display: flex; align-items: flex-end; }
  .banner {
    position: absolute; inset: 0; z-index: -1; width: 100%; height: 100%; object-fit: cover; object-position: center 25%;
    mask-image: linear-gradient(to bottom, #000 35%, transparent 100%);
  }
  /* Keeps the name and text readable over bright banners. */
  .artist::before {
    content: ""; position: absolute; inset: 0; z-index: -1;
    background: linear-gradient(to top, var(--bg) 0%, rgba(11, 11, 15, 0.4) 55%, transparent 100%), linear-gradient(to right, rgba(11, 11, 15, 0.7) 0%, transparent 60%);
  }
  .artist .subtitle, .artist .description { color: rgba(255, 255, 255, 0.78); }
  .content { display: flex; align-items: flex-end; gap: 32px; min-width: 0; }
  .cover { width: 232px; height: 232px; flex: none; border-radius: 10px; overflow: hidden; background: var(--surface-hover); box-shadow: 0 24px 60px rgba(0, 0, 0, 0.55); }
  .cover img { width: 100%; height: 100%; object-fit: cover; }
  .info { min-width: 0; display: grid; gap: 6px; }
  h1 { margin: 0; font-size: clamp(32px, 4.2vw, 56px); font-weight: 800; line-height: 1.05; letter-spacing: -0.03em; }
  .artist h1 { font-size: clamp(44px, 6vw, 84px); text-shadow: 0 4px 30px rgba(0, 0, 0, 0.5); }
  .subtitle { font-size: 14px; color: var(--text-muted); }
  .second { font-size: 13px; color: var(--text-faint); }
  .description {
    max-width: 720px; padding: 0; text-align: left; font-size: 13.5px; line-height: 1.55; color: var(--text-muted);
    display: -webkit-box; -webkit-line-clamp: 2; line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; cursor: pointer;
  }
  .description.expanded { -webkit-line-clamp: unset; line-clamp: unset; }
  .actions { display: flex; gap: 10px; margin-top: 12px; }
  .actions button {
    display: inline-flex; align-items: center; gap: 8px;
    height: 42px; padding: 0 22px; border-radius: 21px;
    font-weight: 700; font-size: 14px;
    transition: transform 0.15s var(--ease), background 0.15s;
  }
  .actions button:hover { transform: scale(1.03); }
  .primary { background: #fff; color: #000; }
  .secondary { background: rgba(255, 255, 255, 0.1); color: var(--text); border: 1px solid var(--border); }
  .secondary:hover { background: rgba(255, 255, 255, 0.16); }
</style>
