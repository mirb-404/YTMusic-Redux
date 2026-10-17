<script lang="ts">
  import { onMount } from "svelte";
  import { formatTime } from "../lib/api";

  // Seek bar whose dot leaves a rippling wave trail behind it while music plays
  // (Samsung-style wake). The trail is strongest right behind the dot, fades with distance,
  // and settles flat when playback pauses.
  let {
    value,
    max,
    playing,
    disabled = false,
    height = 32,
    onseek
  }: { value: number; max: number; playing: boolean; disabled?: boolean; height?: number; onseek: (seconds: number) => void } = $props();

  const TRAIL = 150; // px of wake behind the dot
  const DECAY = 55; // px for the wake to fade to ~37%
  // Layered strands: wavelength (px), ripple speed (rad/s), amplitude scale, width, opacity.
  const STRANDS = [
    { wavelength: 26, speed: 7.5, amp: 1, width: 2.5, alpha: 1, color: "accent" },
    { wavelength: 34, speed: 4.5, amp: 0.55, width: 1.5, alpha: 0.35, color: "white" }
  ] as const;

  let canvas: HTMLCanvasElement;
  let wrap: HTMLDivElement;
  let dragging = $state<number | null>(null);
  let hoverX = $state<number | null>(null);
  let width = $state(0);

  const fraction = $derived(max > 0 ? Math.min(1, Math.max(0, (dragging ?? value) / max)) : 0);
  // Restarts the draw loop, which stops itself once nothing moves.
  let wake = () => {};
  $effect(() => {
    void [fraction, playing, disabled, dragging, hoverX, width];
    wake();
  });

  const hoverTime = $derived(hoverX !== null && max > 0 ? (hoverX / Math.max(1, width - 16)) * max : null);

  onMount(() => {
    const ctx = canvas.getContext("2d")!;
    let energy = playing ? 1 : 0;
    let time = 0;
    let last = performance.now();
    let frame = 0;
    let running = false;

    const style = getComputedStyle(document.documentElement);
    const colors = {
      accent: style.getPropertyValue("--accent").trim() || "#f0445f",
      white: "#ffffff"
    };

    const resize = new ResizeObserver(() => (width = wrap.clientWidth));
    resize.observe(wrap);

    const draw = (now: number) => {
      const dt = Math.min(0.1, (now - last) / 1000);
      last = now;
      const dpr = window.devicePixelRatio || 1;
      const w = wrap.clientWidth;
      const h = height;
      if (canvas.width !== Math.round(w * dpr) || canvas.height !== Math.round(h * dpr)) {
        canvas.width = Math.round(w * dpr);
        canvas.height = Math.round(h * dpr);
      }
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
      ctx.clearRect(0, 0, w, h);

      // Wake energy eases in/out so play/pause never snaps.
      const target = playing && !disabled ? 1 : 0;
      energy += (target - energy) * Math.min(1, dt * 5);
      time += dt * (0.25 + 0.75 * energy);

      const cy = h / 2;
      const pad = 8;
      const x = pad + (w - pad * 2) * fraction;
      const active = dragging !== null || hoverX !== null;
      const maxAmp = Math.min(9, h / 2 - 5);
      ctx.lineCap = "round";
      ctx.lineJoin = "round";

      // Remaining track
      ctx.lineWidth = 3;
      ctx.strokeStyle = "rgba(255,255,255,0.14)";
      ctx.beginPath();
      ctx.moveTo(Math.min(x + 10, w - pad), cy);
      ctx.lineTo(w - pad, cy);
      ctx.stroke();

      if (hoverX !== null && dragging === null && pad + hoverX > x + 10) {
        ctx.strokeStyle = "rgba(255,255,255,0.3)";
        ctx.beginPath();
        ctx.moveTo(x + 10, cy);
        ctx.lineTo(pad + hoverX, cy);
        ctx.stroke();
      }

      // Played line; it recedes under the wake while playing.
      if (x > pad) {
        ctx.strokeStyle = colors.accent;
        ctx.lineWidth = 3;
        ctx.globalAlpha = 1 - 0.5 * energy;
        ctx.beginPath();
        ctx.moveTo(pad, cy);
        ctx.lineTo(x, cy);
        ctx.stroke();
        ctx.globalAlpha = 1;
      }

      // Wave trail: ripples travel backwards away from the dot and fade with distance.
      if (energy > 0.01 && x > pad + 2) {
        const length = Math.min(TRAIL, x - pad);
        for (const strand of STRANDS) {
          const k = (Math.PI * 2) / strand.wavelength;
          const gradient = ctx.createLinearGradient(x - length, 0, x, 0);
          gradient.addColorStop(0, "rgba(0,0,0,0)");
          gradient.addColorStop(1, colors[strand.color]);
          ctx.strokeStyle = gradient;
          ctx.lineWidth = strand.width;
          ctx.globalAlpha = strand.alpha * energy;
          ctx.beginPath();
          for (let d = 0; d <= length; d += 1) {
            // Rises from zero at the dot, peaks just behind it, then decays.
            const envelope = (1 - Math.exp(-d / 9)) * Math.exp(-d / DECAY);
            const y = cy + Math.sin(k * d - time * strand.speed) * maxAmp * strand.amp * envelope * energy;
            if (d === 0) ctx.moveTo(x, y);
            else ctx.lineTo(x - d, y);
          }
          ctx.stroke();
        }
        ctx.globalAlpha = 1;
      }

      // Dot
      ctx.fillStyle = "#fff";
      ctx.beginPath();
      ctx.arc(x, cy, active ? 7 : 6, 0, Math.PI * 2);
      ctx.fill();

      // Keep animating only while the wake moves; a paused, idle bar costs nothing.
      if (energy > 0.01 || target === 1) {
        frame = requestAnimationFrame(draw);
      } else {
        running = false;
      }
    };
    wake = () => {
      if (running) return;
      running = true;
      last = performance.now();
      frame = requestAnimationFrame(draw);
    };
    wake();

    return () => {
      cancelAnimationFrame(frame);
      resize.disconnect();
    };
  });

  function positionOf(e: PointerEvent) {
    const rect = wrap.getBoundingClientRect();
    return Math.max(0, Math.min(rect.width - 16, e.clientX - rect.left - 8));
  }

  function secondsAt(e: PointerEvent) {
    const rect = wrap.getBoundingClientRect();
    return (positionOf(e) / Math.max(1, rect.width - 16)) * max;
  }

  function down(e: PointerEvent) {
    if (disabled || max <= 0) return;
    wrap.setPointerCapture(e.pointerId);
    dragging = secondsAt(e);
  }

  function move(e: PointerEvent) {
    hoverX = positionOf(e);
    if (dragging !== null) dragging = secondsAt(e);
  }

  function up() {
    if (dragging === null) return;
    const target = dragging;
    dragging = null;
    onseek(Math.floor(target));
  }

  function key(e: KeyboardEvent) {
    if (disabled || max <= 0) return;
    const step = e.shiftKey ? 30 : 5;
    if (e.key === "ArrowRight") onseek(Math.min(max, Math.floor(value + step)));
    else if (e.key === "ArrowLeft") onseek(Math.max(0, Math.floor(value - step)));
    else return;
    e.preventDefault();
  }
</script>

<div
  class="wave"
  class:disabled
  bind:this={wrap}
  style:height="{height}px"
  role="slider"
  tabindex={disabled ? -1 : 0}
  aria-label="Seek"
  aria-valuemin={0}
  aria-valuemax={Math.floor(max)}
  aria-valuenow={Math.floor(dragging ?? value)}
  aria-valuetext={formatTime(dragging ?? value)}
  onpointerdown={down}
  onpointermove={move}
  onpointerup={up}
  onpointercancel={up}
  onpointerleave={() => (hoverX = null)}
  onkeydown={key}
>
  <canvas bind:this={canvas} style:height="{height}px"></canvas>
  {#if (dragging !== null || hoverTime !== null) && !disabled && max > 0}
    {@const shown = dragging ?? hoverTime ?? 0}
    <span class="tip" style:left="{8 + (shown / max) * Math.max(1, width - 16)}px">{formatTime(shown)}</span>
  {/if}
</div>

<style>
  .wave {
    position: relative;
    width: 100%;
    cursor: pointer;
    touch-action: none;
    border-radius: 6px;
  }
  .wave.disabled { cursor: default; opacity: 0.5; }
  canvas { display: block; width: 100%; }
  .tip {
    position: absolute;
    bottom: calc(100% + 2px);
    transform: translateX(-50%);
    padding: 3px 7px;
    border-radius: 6px;
    background: rgba(20, 20, 28, 0.95);
    border: 1px solid var(--border);
    font-size: 11px;
    font-variant-numeric: tabular-nums;
    pointer-events: none;
    white-space: nowrap;
  }
</style>
