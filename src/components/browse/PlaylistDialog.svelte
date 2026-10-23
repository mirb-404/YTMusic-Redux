<script lang="ts">
  import { fade, scale } from "svelte/transition";
  import { cubicOut } from "svelte/easing";
  import Icon from "../Icon.svelte";
  import { browseTo, music, router, toast, ui, type PlaylistTarget } from "../../lib/music.svelte";

  // Saves a track or list to one of the user's playlists, or creates a playlist.
  let { onchange }: { onchange: () => void } = $props();

  const dialog = $derived(ui.dialog);
  let targets = $state<PlaylistTarget[] | null>(null);
  let creating = $state(false);
  let title = $state("");
  let description = $state("");
  let privacy = $state("PRIVATE");
  let busy = $state(false);
  let input = $state<HTMLInputElement>();

  $effect(() => {
    const d = dialog;
    targets = null;
    creating = !!d?.create;
    title = "";
    description = "";
    if (d?.target) music.playlistTargets(d.target.videoId, d.target.playlistId).then((t) => (targets = t)).catch((e) => toast(String(e)));
  });

  $effect(() => {
    if (creating) queueMicrotask(() => input?.focus());
  });

  function close() {
    ui.dialog = null;
  }

  async function save(p: PlaylistTarget) {
    const t = dialog?.target;
    if (!t || busy) return;
    busy = true;
    try {
      await music.addToPlaylist(p.playlistId, t.videoId, t.videoId ? null : t.playlistId);
      toast(`Saved to ${p.title}`);
      close();
    } catch (e) {
      toast(String(e));
    } finally {
      busy = false;
    }
  }

  async function create() {
    if (!title.trim() || busy) return;
    busy = true;
    const t = dialog?.target ?? null;
    try {
      const id = await music.createPlaylist(title, description, privacy, t?.videoId ?? null, t && !t.videoId ? t.playlistId : null);
      toast(`Created ${title.trim()}`);
      close();
      onchange();
      router.go({ kind: "browse", endpoint: browseTo(`VL${id}`) });
    } catch (e) {
      toast(String(e));
    } finally {
      busy = false;
    }
  }
</script>

{#if dialog}
  <div class="backdrop" role="presentation" transition:fade={{ duration: 150 }} onpointerdown={close}></div>
  <div class="dialog" role="dialog" aria-modal="true" transition:scale={{ start: 0.96, duration: 180, easing: cubicOut }}>
    <header>
      <h2>{creating ? "New playlist" : "Save to playlist"}</h2>
      <button class="x" title="Close" onclick={close}><Icon name="close" size={18} /></button>
    </header>
    {#if dialog.target}<p class="what">{dialog.target.title}</p>{/if}

    {#if creating}
      <form onsubmit={(e) => { e.preventDefault(); create(); }}>
        <label>Title<input bind:this={input} bind:value={title} maxlength="150" placeholder="My playlist" /></label>
        <label>Description<textarea bind:value={description} rows="2" maxlength="5000"></textarea></label>
        <div class="privacy">
          {#each [["PRIVATE", "Private"], ["UNLISTED", "Unlisted"], ["PUBLIC", "Public"]] as [value, label] (value)}
            <button type="button" class:active={privacy === value} onclick={() => (privacy = value)}>{label}</button>
          {/each}
        </div>
        <div class="actions">
          {#if dialog.target}<button type="button" class="ghost" onclick={() => (creating = false)}>Back</button>{/if}
          <button type="submit" class="primary" disabled={!title.trim() || busy}>Create</button>
        </div>
      </form>
    {:else}
      <button class="new" onclick={() => (creating = true)}><span class="plus"><Icon name="add" size={20} /></span>New playlist</button>
      <div class="list">
        {#if targets === null}
          {#each [0, 1, 2, 3] as i (i)}<span class="sk"></span>{/each}
        {:else}
          {#each targets as p (p.playlistId)}
            <button class="target" disabled={busy} onclick={() => save(p)}>
              <span class="name">{p.title}</span>
              {#if p.contains}<span class="saved"><Icon name="check" size={16} />Saved</span>{/if}
            </button>
          {/each}
        {/if}
      </div>
    {/if}
  </div>
{/if}

<style>
  .backdrop { position: fixed; inset: 0; z-index: 95; background: rgba(0, 0, 0, 0.7); }
  .dialog {
    position: fixed; z-index: 96; left: 50%; top: 50%; translate: -50% -50%;
    width: min(440px, calc(100vw - 48px)); max-height: min(620px, calc(100vh - 120px));
    display: flex; flex-direction: column;
    padding: 24px; border-radius: var(--radius);
    background: #282828;
    box-shadow: var(--shadow-pop);
  }
  header { display: flex; align-items: center; justify-content: space-between; }
  h2 { margin: 0; font-size: 24px; font-weight: 800; letter-spacing: -0.02em; }
  .x { width: 32px; height: 32px; border-radius: 50%; display: grid; place-items: center; color: var(--text-muted); }
  .x:hover { background: var(--surface-hover); color: var(--text); }
  .what { margin: 4px 0 12px; font-size: 13px; color: var(--text-muted); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .new { display: flex; align-items: center; gap: 12px; padding: 8px; border-radius: 4px; font-weight: 600; transition: background var(--dur-fast) var(--ease); }
  .new:hover { background: var(--surface-hover); }
  .plus { width: 44px; height: 44px; border-radius: 4px; display: grid; place-items: center; background: var(--accent-gradient); color: #fff; }
  .list { overflow-y: auto; min-height: 0; margin-top: 4px; display: grid; gap: 2px; }
  .target { display: flex; align-items: center; gap: 12px; padding: 12px 10px; border-radius: 4px; text-align: left; transition: background var(--dur-fast) var(--ease); }
  .target:hover:not(:disabled) { background: var(--surface-hover); }
  .name { flex: 1; min-width: 0; font-weight: 600; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .saved { display: inline-flex; align-items: center; gap: 4px; font-size: 12px; color: var(--success); }
  .sk { height: 40px; border-radius: 8px; background: var(--surface); }
  form { display: grid; gap: 14px; margin-top: 8px; }
  label { display: grid; gap: 6px; font-size: 12.5px; color: var(--text-muted); }
  input, textarea {
    padding: 10px 12px; border-radius: 4px; border: 1px solid transparent;
    background: rgba(255, 255, 255, 0.1); color: var(--text); font: inherit; font-size: 14px; outline: none; resize: none;
  }
  input:focus, textarea:focus { border-color: rgba(255, 255, 255, 0.5); background: #333; }
  .privacy { display: flex; gap: 2px; padding: 3px; border-radius: 999px; background: rgba(255, 255, 255, 0.08); }
  .privacy button { flex: 1; padding: 7px; border-radius: 999px; transition: background var(--dur) var(--ease), color var(--dur) var(--ease); font-size: 13px; font-weight: 600; color: var(--text-muted); }
  .privacy button.active { background: #fff; color: #000; }
  .actions { display: flex; justify-content: flex-end; gap: 8px; }
  .actions button { height: 44px; padding: 0 28px; border-radius: 999px; font-weight: 700; transition: transform var(--dur-fast) var(--ease); }
  .actions button:hover:not(:disabled) { transform: scale(1.04); }
  .ghost { color: var(--text-muted); }
  .ghost:hover { background: var(--surface-hover); color: var(--text); }
  .primary { background: var(--accent); color: var(--accent-ink); }
  .primary:disabled { opacity: 0.4; }
</style>
