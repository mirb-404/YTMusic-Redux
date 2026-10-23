<script lang="ts">
  import { onMount } from "svelte";
  import Icon from "../components/Icon.svelte";
  import { api } from "../lib/api";

  let appName = $state("");
  let code = $state("");
  let expired = $state(false);

  onMount(async () => {
    try {
      ({ appName, code } = await api.companionAuthInfo());
    } catch {
      expired = true;
    }
  });

  const respond = (allowed: boolean) => api.companionAuthResult(allowed);
</script>

<div class="auth" data-tauri-drag-region>
  <div class="badge"><Icon name="integrations" size={30} /></div>
  {#if expired}
    <h1>Request expired</h1>
    <p class="sub">This authorization request is no longer active.</p>
  {:else}
    <h1>Companion authorization</h1>
    <p class="sub"><b>{appName}</b> wants to control YTMusic Redux</p>
    <p class="hint">Make sure this code matches the one shown by <b>{appName}</b></p>
    <div class="code" aria-label="Authorization code {code}">
      {#each code.split("") as digit, i (i)}<span>{digit}</span>{/each}
    </div>
    <div class="actions">
      <button class="btn deny" onclick={() => respond(false)}>Deny</button>
      <button class="btn primary" onclick={() => respond(true)}>Allow</button>
    </div>
  {/if}
</div>

<style>
  .auth {
    position: relative;
    height: 100%;
    display: flex; flex-direction: column; align-items: center; justify-content: center;
    padding: 28px; text-align: center; overflow: hidden;
    border: 1px solid var(--border);
  }
  .badge {
    position: relative; width: 56px; height: 56px; border-radius: 14px;
    display: grid; place-items: center; color: #fff;
    background: var(--accent-gradient);
  }
  h1 { position: relative; font-size: 22px; margin: 18px 0 6px; }
  .sub { position: relative; margin: 0; font-size: 15px; color: var(--text-muted); }
  .sub b, .hint b { color: var(--text); }
  .hint { position: relative; margin: 22px 0 10px; font-size: 13px; color: var(--text-muted); }
  .code { position: relative; display: flex; gap: 10px; margin-bottom: 26px; }
  .code span {
    width: 52px; height: 64px;
    display: grid; place-items: center;
    font-size: 32px; font-weight: 800; font-variant-numeric: tabular-nums;
    border-radius: var(--radius); background: var(--bg-elevated); border: 1px solid var(--border);
  }
  .actions { position: relative; display: flex; gap: 12px; }
  .actions .btn { min-width: 120px; justify-content: center; padding: 10px 16px; }
  .deny:hover { color: var(--danger); }
</style>
