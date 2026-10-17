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
    background: rgba(255, 255, 255, 0.14);
    transition: background 0.2s var(--ease);
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
    transition: transform 0.25s var(--ease);
  }
  .on .knob { transform: translateX(18px); }
  .toggle:active .knob { width: 22px; }
  .on:active .knob { transform: translateX(14px); }
</style>
