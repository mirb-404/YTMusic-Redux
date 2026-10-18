<script lang="ts">
  import type { Snippet } from "svelte";

  let {
    title,
    description = "",
    restart = false,
    danger = false,
    nested = false,
    beta = false,
    children
  }: { title: string; description?: string; restart?: boolean; danger?: boolean; nested?: boolean; beta?: boolean; children?: Snippet } = $props();
</script>

<div class="row" class:nested>
  <div class="text">
    <div class="title">
      {title}
      {#if restart}<span class="tag">Restart</span>{/if}
      {#if danger}<span class="tag danger">Caution</span>{/if}
      {#if beta}<span class="tag beta">Beta</span>{/if}
    </div>
    {#if description}<div class="description">{description}</div>{/if}
  </div>
  <div class="control">{@render children?.()}</div>
</div>

<style>
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 24px;
    padding: 14px 18px;
    border-top: 1px solid var(--border);
  }
  .row:first-child { border-top: none; }
  .row.nested { padding-left: 34px; background: rgba(255, 255, 255, 0.015); }
  .text { min-width: 0; }
  .title { font-weight: 600; display: flex; align-items: center; gap: 8px; }
  .description { color: var(--text-muted); font-size: 12.5px; margin-top: 2px; }
  .control { flex: none; display: flex; align-items: center; gap: 8px; }
  .tag {
    font-size: 10.5px;
    font-weight: 700;
    letter-spacing: 0.4px;
    text-transform: uppercase;
    padding: 2px 6px;
    border-radius: 5px;
    background: rgba(255, 181, 71, 0.14);
    color: var(--warning);
  }
  .tag.danger { background: rgba(255, 107, 107, 0.14); color: var(--danger); }
  .tag.beta { background: color-mix(in srgb, var(--accent) 16%, transparent); color: var(--accent); }
</style>
