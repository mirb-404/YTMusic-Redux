<script lang="ts">
  import { fly } from "svelte/transition";
  import { cubicOut } from "svelte/easing";
  import { ui } from "../../lib/music.svelte";
</script>

{#if ui.toast}
  {#key ui.toast.id}
    <div class="toast" role="status" in:fly={{ y: 16, duration: 260, easing: cubicOut }} out:fly={{ y: 8, duration: 180 }}>{ui.toast.text}</div>
  {/key}
{/if}

<style>
  .toast {
    position: absolute; left: 50%; bottom: 20px; z-index: 80; translate: -50% 0;
    max-width: min(520px, calc(100% - 48px)); padding: 11px 20px; border-radius: 22px;
    background: #fff; color: #000; font-weight: 600; font-size: 13.5px;
    box-shadow: 0 16px 40px rgba(0, 0, 0, 0.5);
    white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
    pointer-events: none;
  }
</style>
