<script lang="ts">
  import { fly } from "svelte/transition";
  import { cubicOut } from "svelte/easing";
  import Icon from "../Icon.svelte";
  import { api } from "../../lib/api";
  import { art, auth, music, signOut, toast } from "../../lib/music.svelte";

  // The top bar's account control: who is signed in (with sign out), a sign-in button when
  // signed out, and a way back while Google's sign-in pages are on screen.
  let { signingIn }: { signingIn: boolean } = $props();

  let open = $state(false);
  let busy = $state(false);
  const account = $derived(auth.account);
  const photo = $derived(account ? art(account.photo, 32) : null);
  const initial = $derived(account?.name.trim().charAt(0).toUpperCase() ?? "");

  async function leave() {
    open = false;
    busy = true;
    try {
      await signOut();
      toast("Signed out of YouTube Music");
    } catch (e) {
      toast(`Could not sign out: ${e}`);
    } finally {
      busy = false;
    }
  }
</script>

{#if signingIn}
  <button class="pill" title="Leave Google's sign-in and return to the app" onclick={() => api.ytmNavigateHome()}><Icon name="close" size={16} />Cancel sign-in</button>
{:else if account === null}
  <button class="pill light" onclick={() => music.signIn()}><Icon name="account" size={18} />Sign in</button>
{:else if account}
  <div class="account">
    <button class="avatar" class:open class:photo title={account.name ? `Signed in as ${account.name}` : "Signed in"} aria-haspopup="menu" aria-expanded={open} disabled={busy} onclick={() => (open = !open)}>
      {#if photo}<img src={photo} alt="" />{:else if initial}<span>{initial}</span>{:else}<Icon name="account" size={20} />{/if}
    </button>
    {#if open}
      <div class="scrim" role="presentation" onpointerdown={() => (open = false)}></div>
      <div class="menu" role="menu" transition:fly={{ y: -6, duration: 160, easing: cubicOut }}>
        <div class="who">
          <span class="label">Signed in{account.name ? " as" : ""}</span>
          {#if account.name}<span class="name">{account.name}</span>{/if}
          {#if account.handle}<span class="handle">{account.handle}</span>{/if}
        </div>
        <button role="menuitem" onclick={leave}><Icon name="close" size={18} />Sign out</button>
      </div>
    {/if}
  </div>
{/if}

<style>
  .pill {
    display: inline-flex; align-items: center; gap: 8px; height: 32px; padding: 0 14px; margin-right: 6px;
    border-radius: 999px; background: var(--bg-highlight); color: var(--text); font-size: 13px; font-weight: 700; white-space: nowrap;
    transition: transform var(--dur-fast) var(--ease), background var(--dur-fast) var(--ease);
  }
  .pill.light { background: #fff; color: #000; }
  .pill:hover { transform: scale(1.04); }
  .pill:active { transform: scale(0.97); }
  .account { position: relative; margin-right: 6px; }
  .avatar {
    width: 32px; height: 32px; padding: 0; border-radius: 50%; overflow: hidden;
    display: grid; place-items: center;
    background: var(--accent); color: var(--accent-ink); font-weight: 800; font-size: 14px;
    outline: 4px solid var(--bg-elevated);
    transition: transform var(--dur-fast) var(--ease);
  }
  .avatar:hover:not(:disabled), .avatar.open { transform: scale(1.06); }
  .avatar:disabled { opacity: 0.5; }
  .avatar.photo { background: var(--bg-highlight); }
  /* Slight zoom so the photo fills the circle edge to edge, with no ring of background. */
  .avatar img { display: block; width: 100%; height: 100%; object-fit: cover; transform: scale(1.12); }
  .scrim { position: fixed; inset: 0; z-index: 40; }
  .menu {
    position: absolute; right: 0; top: calc(100% + 10px); z-index: 41; min-width: 220px; padding: 4px;
    border-radius: var(--radius-sm); background: var(--bg-elevated); box-shadow: var(--shadow-pop);
  }
  .who { display: grid; gap: 2px; padding: 10px 12px 12px; border-bottom: 1px solid var(--border); margin-bottom: 4px; }
  .label { font-size: 12px; color: var(--text-muted); }
  .name { font-size: 14px; font-weight: 700; }
  .handle { font-size: 12.5px; color: var(--text-muted); }
  .name, .handle { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; max-width: 260px; }
  .menu button {
    width: 100%; display: flex; align-items: center; gap: 12px; height: 40px; padding: 0 12px;
    border-radius: 2px; font-size: 14px; text-align: left;
  }
  .menu button:hover { background: var(--surface-hover); }
</style>
