<script lang="ts">
  import Icon from "./Icon.svelte";

  let { value, failed = false, onchange }: { value: string; failed?: boolean; onchange: (accelerator: string) => void } = $props();
  let recording = $state(false);

  const MODIFIER_CODES = ["ControlLeft", "ControlRight", "AltLeft", "AltRight", "ShiftLeft", "ShiftRight", "MetaLeft", "MetaRight"];
  const MEDIA_KEYS = new Set(["MediaPlayPause", "MediaTrackNext", "MediaTrackPrevious", "MediaStop", "AudioVolumeUp", "AudioVolumeDown", "AudioVolumeMute"]);

  // Accelerators use KeyboardEvent.code names, which the global-hotkey parser accepts.
  function record(e: KeyboardEvent) {
    e.preventDefault();
    if (e.code === "Escape") {
      recording = false;
      return;
    }
    if (MODIFIER_CODES.includes(e.code)) return;
    const parts = [];
    if (e.ctrlKey) parts.push("Ctrl");
    if (e.altKey) parts.push("Alt");
    if (e.shiftKey) parts.push("Shift");
    if (e.metaKey) parts.push("Super");
    // Plain keys would hijack typing everywhere; require a modifier unless it is a media/function key.
    if (!parts.length && !MEDIA_KEYS.has(e.code) && !/^F\d+$/.test(e.code)) return;
    parts.push(e.code);
    recording = false;
    onchange(parts.join("+"));
  }

  const pretty = (accel: string) =>
    accel
      .split("+")
      .map((p) => p.replace(/^Key/, "").replace(/^Digit/, "").replace(/^Arrow/, "").replace(/^Media/, "").replace(/^Audio/, ""))
      .join(" + ");
</script>

<div class="keybind" class:failed>
  <button class="field" class:recording onclick={() => (recording = true)} onkeydown={recording ? record : undefined} onblur={() => (recording = false)}>
    {#if recording}
      <span class="hint">Press keys…</span>
    {:else if value}
      {#each pretty(value).split(" + ") as key, i (i)}<kbd>{key}</kbd>{/each}
    {:else}
      <span class="hint">Not set</span>
    {/if}
  </button>
  {#if value}
    <button class="icon-btn" title="Clear" onclick={() => onchange("")}><Icon name="close" size={16} /></button>
  {/if}
  {#if failed}
    <span class="warn" title="Failed to register keybind. Does another application have this keybind?"><Icon name="warning" size={18} /></span>
  {/if}
</div>

<style>
  .keybind { display: flex; align-items: center; gap: 4px; }
  .field {
    min-width: 180px;
    height: 34px;
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 0 10px;
    border-radius: var(--radius-sm);
    background: var(--surface);
    border: 1px solid var(--border);
    transition: border-color 0.15s, box-shadow 0.15s;
  }
  .field:hover { border-color: rgba(255, 255, 255, 0.18); }
  .field.recording { border-color: var(--accent); }
  .failed .field { border-color: var(--warning); }
  kbd {
    font-family: inherit;
    font-size: 12px;
    font-weight: 600;
    padding: 2px 7px;
    border-radius: 5px;
    background: var(--surface-active);
    border-bottom: 2px solid rgba(0, 0, 0, 0.4);
  }
  .hint { color: var(--text-faint); font-size: 13px; }
  .recording .hint { color: var(--accent); }
  .warn { color: var(--warning); display: grid; }
</style>
