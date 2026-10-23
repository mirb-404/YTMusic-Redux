<script lang="ts">
  import Icon from "../Icon.svelte";
  import { api, type PlayerState } from "../../lib/api";
  import { art } from "../../lib/music.svelte";

  // Announces the next track during the last seconds of the current one, over the app's
  // pages (the YTM page shows its own over the video view).
  let { player, elapsed }: { player: PlayerState | null; elapsed: number } = $props();

  const SECONDS = 15;
  // Circumference of the countdown ring (r = 21 in a 48px box).
  const RING = 2 * Math.PI * 21;

  const video = $derived(player?.video ?? null);
  const next = $derived.by(() => {
    const q = player?.player.queue;
    if (!q || q.repeatMode === 2 || q.selectedItemIndex < 0) return null;
    return q.items[q.selectedItemIndex + 1] ?? q.automixItems[0] ?? (q.repeatMode === 1 ? q.items[0] : null) ?? null;
  });
  const left = $derived(video ? video.durationSeconds - elapsed : Infinity);
  let dismissed = $state("");
  const show = $derived(
    !!next && !!video && !video.isLive && !player?.player.adPlaying && video.durationSeconds > SECONDS * 3 && left > 0.5 && left <= SECONDS && dismissed !== video.id
  );
  const cover = $derived(art(next?.thumbnails, 48));
</script>

<div class="pill" class:show aria-hidden={!show}>
  {#if next}
    {#if cover}<img class="glow" src={cover} alt="" />{/if}
    <div class="art">
      {#if cover}<img src={cover} alt="" />{/if}
      <svg class="ring" viewBox="0 0 48 48" aria-hidden="true">
        <circle cx="24" cy="24" r="21" />
        <circle class="fill" cx="24" cy="24" r="21" stroke-dasharray={RING} stroke-dashoffset={RING * (1 - Math.max(0, left) / SECONDS)} />
      </svg>
    </div>
    <div class="text">
      <div class="label">Up next<span>{Math.max(0, Math.ceil(left))}s</span></div>
      <div class="title">{next.title}</div>
      <div class="artist">{next.author}</div>
    </div>
    <button class="play" title="Play now" tabindex={show ? 0 : -1} onclick={() => api.command("next")}><Icon name="next" size={20} /></button>
    <button class="dismiss" title="Dismiss" tabindex={show ? 0 : -1} onclick={() => (dismissed = video?.id ?? "")}><Icon name="close" size={16} /></button>
  {/if}
</div>

<style>
  .pill {
    position: absolute; left: 50%; bottom: 22px; z-index: 15;
    display: flex; align-items: center; gap: 14px; height: 64px;
    max-width: min(520px, calc(100% - 48px)); padding: 8px 10px 8px 8px;
    border-radius: 999px; overflow: hidden; isolation: isolate;
    background: rgba(18, 18, 18, 0.72); backdrop-filter: blur(28px) saturate(1.8);
    border: 1px solid rgba(255, 255, 255, 0.09);
    box-shadow: 0 24px 60px rgba(0, 0, 0, 0.55), inset 0 1px 0 rgba(255, 255, 255, 0.07);
    opacity: 0; transform: translate(-50%, 28px) scale(0.92); pointer-events: none;
    transition: opacity 0.35s ease, transform 0.55s cubic-bezier(0.2, 0.9, 0.25, 1.12);
  }
  .pill.show { opacity: 1; transform: translate(-50%, 0) scale(1); pointer-events: auto; }
  .glow { position: absolute; inset: -30px; z-index: -1; width: calc(100% + 60px); height: calc(100% + 60px); object-fit: cover; filter: blur(30px) saturate(1.8); opacity: 0.35; }
  .art { position: relative; flex: none; width: 48px; height: 48px; display: grid; place-items: center; }
  .art img { width: 40px; height: 40px; border-radius: 50%; object-fit: cover; box-shadow: 0 4px 14px rgba(0, 0, 0, 0.4); }
  .ring { position: absolute; inset: 0; transform: rotate(-90deg); }
  .ring circle { fill: none; stroke: rgba(255, 255, 255, 0.14); stroke-width: 2.5; }
  .ring .fill { stroke: var(--accent); stroke-linecap: round; transition: stroke-dashoffset 0.3s linear; }
  .text { min-width: 0; display: grid; gap: 1px; padding-right: 4px; }
  .label { font-size: 10.5px; font-weight: 700; letter-spacing: 1.2px; text-transform: uppercase; color: rgba(255, 255, 255, 0.55); }
  .label span { margin-left: 6px; letter-spacing: 0; text-transform: none; font-weight: 600; font-variant-numeric: tabular-nums; color: rgba(255, 255, 255, 0.8); }
  .title { font-size: 14px; font-weight: 600; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .artist { font-size: 12.5px; color: rgba(255, 255, 255, 0.62); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  button { flex: none; display: grid; place-items: center; border-radius: 50%; }
  .play { width: 40px; height: 40px; background: var(--accent); color: var(--accent-ink); transition: transform var(--dur-fast) var(--ease); }
  .play:hover { transform: scale(1.07); }
  .dismiss { width: 28px; height: 28px; margin-left: -4px; color: rgba(255, 255, 255, 0.55); }
  .dismiss:hover { color: #fff; background: rgba(255, 255, 255, 0.1); }
  @media (prefers-reduced-motion: reduce) { .pill { transition: opacity 0.2s; transform: translate(-50%, 0); } }
</style>
