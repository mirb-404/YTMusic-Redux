<script lang="ts">
  import { scale } from "svelte/transition";
  import { cubicOut } from "svelte/easing";
  import Icon from "../Icon.svelte";
  import type { IconName } from "../../lib/icons";
  import { rate, runEntry, ui, type MenuEntry } from "../../lib/music.svelte";

  // YTM's menu icon names, drawn with the app's icons.
  const ICONS: Record<string, IconName> = {
    QUEUE_PLAY_NEXT: "playNext",
    ADD_TO_REMOTE_QUEUE: "queue",
    BOOKMARK_BORDER: "bookmarkBorder",
    LIBRARY_ADD: "bookmarkBorder",
    BOOKMARK: "bookmark",
    LIBRARY_REMOVE: "bookmark",
    ADD_TO_PLAYLIST: "playlistAdd",
    ARTIST: "account",
    ALBUM: "album",
    SHARE: "share",
    MIX: "radio",
    MUSIC_SHUFFLE: "shuffle",
    KEEP: "pin",
    KEEP_OFF: "pin",
    DELETE: "delete",
    REMOVE: "delete",
    PEOPLE_GROUP: "people"
  };
  const icon = (name: string): IconName => ICONS[name] ?? "music";

  let el = $state<HTMLElement>();
  let pos = $state({ x: 0, y: 0 });

  // Opens at the pointer, kept inside the window.
  $effect(() => {
    const m = ui.menu;
    if (!m || !el) return;
    const r = el.getBoundingClientRect();
    pos = { x: Math.min(m.x, innerWidth - r.width - 8), y: Math.min(m.y, innerHeight - r.height - 8) };
  });

  function close() {
    ui.menu = null;
  }

  function run(entry: MenuEntry) {
    const m = ui.menu!;
    close();
    runEntry(entry, m.item, m.title, m.entries);
  }

  function onKey(e: KeyboardEvent) {
    if (ui.menu && e.key === "Escape") {
      e.stopPropagation();
      close();
    }
  }
</script>

<svelte:window onkeydown={onKey} onresize={close} />

{#if ui.menu}
  {@const like = ui.menu.item?.like}
  <div class="scrim" role="presentation" onpointerdown={close} oncontextmenu={(e) => { e.preventDefault(); close(); }}></div>
  <div
    class="menu"
    role="menu"
    bind:this={el}
    style:left="{pos.x}px"
    style:top="{pos.y}px"
    transition:scale={{ start: 0.95, duration: 140, easing: cubicOut }}
  >
    {#if ui.menu.title}<div class="title">{ui.menu.title}</div>{/if}
    {#if like}
      <button role="menuitem" onclick={() => { close(); rate(like, like.status === "LIKE" ? "INDIFFERENT" : "LIKE"); }}>
        <Icon name={like.status === "LIKE" ? "like" : "likeOutline"} size={18} />{like.status === "LIKE" ? "Remove from Liked music" : "Like"}
      </button>
    {/if}
    {#each ui.menu.entries as entry, i (i)}
      <button role="menuitem" class:danger={entry.icon === "DELETE" || entry.icon === "REMOVE"} onclick={() => run(entry)}>
        <Icon name={icon(entry.icon)} size={18} />{entry.text || (entry.action.type === "share" ? "Share" : "")}
      </button>
    {/each}
  </div>
{/if}

<style>
  .scrim { position: fixed; inset: 0; z-index: 90; }
  .menu {
    position: fixed; z-index: 91; min-width: 240px; max-width: 320px;
    padding: 6px; border-radius: 12px;
    background: rgba(24, 24, 31, 0.96); border: 1px solid var(--border);
    box-shadow: 0 20px 50px rgba(0, 0, 0, 0.6); backdrop-filter: blur(20px);
    transform-origin: top left;
  }
  .title { padding: 8px 12px 8px; font-size: 12px; font-weight: 700; color: var(--text-faint); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; border-bottom: 1px solid var(--border); margin-bottom: 4px; }
  button { width: 100%; display: flex; align-items: center; gap: 12px; padding: 9px 12px; border-radius: 8px; font-size: 13.5px; text-align: left; color: var(--text); }
  button :global(svg) { color: var(--text-muted); flex: none; }
  button:hover, button:focus-visible { background: var(--surface-hover); }
  .danger, .danger :global(svg) { color: var(--danger); }
</style>
