<script lang="ts">
  let { checked, disabled = false, label, onchange }: { checked: boolean; disabled?: boolean; label: string; onchange: (v: boolean) => void } = $props();
</script>

<button class="toggle" class:on={checked} role="switch" aria-checked={checked} aria-label={label} {disabled} onclick={() => onchange(!checked)}>
  <span class="knob"></span>
</button>

<style>
  .toggle {
    position: relative;
    width: 42px;
    height: 24px;
    flex: none;
    border-radius: 12px;
    background: #727272;
    transition: background var(--dur) var(--ease);
  }
  .toggle.on { background: var(--accent); }
  .toggle:disabled { opacity: 0.4; cursor: not-allowed; }
  .knob {
    position: absolute;
    top: 3px;
    left: 3px;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: #fff;
    transition: transform var(--dur) var(--ease-spring), width var(--dur-fast) var(--ease);
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.4);
  }
  .on .knob { transform: translateX(18px); }
  .toggle:active .knob { width: 22px; }
  .on:active .knob { transform: translateX(14px); }
</style>
