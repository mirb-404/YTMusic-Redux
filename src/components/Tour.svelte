<script lang="ts">
  import { onMount } from "svelte";
  import { fade } from "svelte/transition";
  import { cubicOut } from "svelte/easing";
  import Icon from "./Icon.svelte";

  // First-start tour: a spotlight with a heavy vignette moves between the app's own controls
  // (marked with data-tour) while a card explains each one. Steps whose control is not on
  // screen, e.g. the player bar with Modern UI off, are left out.
  let { onclose }: { onclose: () => void } = $props();

  // `native` limits a step to the app's own pages (true) or YouTube Music's (false).
  type Step = { target?: string; title: string; body: string; native?: boolean };
  const ALL_STEPS: Step[] = [
    { title: "Welcome to YTMusic Redux", body: "A quick look at the controls that make this more than a browser tab. It takes half a minute." },
    { target: "sidebar", title: "Your music", body: "Home, Explore, Library, History and your playlists. The app draws these pages itself, so they open instantly. ＋ makes a new playlist." },
    { target: "nav", title: "Find your way", body: "Back, forward and home move through YouTube Music instantly, without reloading the page." },
    { target: "history", title: "Back and forward", body: "Step through the pages you visited, or use your mouse's side buttons and Alt+Left / Alt+Right." },
    { target: "search", title: "Search", body: "Suggestions appear as you type. Press / anywhere to jump here, then filter results by songs, albums, artists and more." },
    { native: true, title: "Right-click anything", body: "Songs, albums, artists and playlists all have a menu: play next, add to queue, save to a playlist, like, share and more. The ⋯ button opens it too." },
    { target: "plugins", title: "Plugins", body: "Turn Discord presence, Last.fm scrobbling and the companion server on or off in one click." },
    { target: "mini", title: "Mini player", body: "A small always-on-top player for when you're working in other windows." },
    { target: "settings", title: "Settings", body: "Equalizer, ad blocking, playback speed, shortcuts, themes and every other option." },
    { target: "art", native: true, title: "Video view", body: "Playing a song opens it with the music video or artwork, its queue and lyrics. Click the artwork to open it again; back or Esc slides it away." },
    { target: "art", native: false, title: "Player page", body: "Click the artwork to open YouTube Music's player page with the full queue." },
    { target: "controls", title: "Playback", body: "Shuffle, previous, play, next and repeat. Media keys and global shortcuts work too." },
    { target: "seek", title: "The wave", body: "The seek line ripples while music plays. Click or drag anywhere on it to jump." },
    { target: "lyrics", title: "Synced lyrics", body: "Opens Now Playing with lyrics that follow the song. Click a line to jump to it." },
    { target: "video", title: "Video or audio only", body: "Switch music videos off to stream audio only and show the artwork instead, which also saves bandwidth." },
    { target: "queue", native: true, title: "Queue", body: "Opens beside your pages. Click a track to play it, drag to reorder, or remove what you don't want next." },
    { target: "sleep", title: "Sleep timer", body: "Pause the music after a set time. The countdown shows right here." },
    { target: "volume", title: "Volume", body: "Drag the slider, or scroll anywhere over it." },
    { target: "fullscreen", title: "Full screen", body: "F11 goes full screen with every control still in reach. Esc or F11 brings you back." },
    { title: "Keyboard", body: "Space plays and pauses, L likes, Y opens lyrics, Q the queue, Ctrl+Shift+X closes the track. Press ? for every shortcut." },
    { title: "You're all set", body: "Enjoy the music. You can replay this tour any time from Settings → General." }
  ];

  const PAD = 8;
  const CARD_W = 340;

  let steps = $state<Step[]>([]);
  let index = $state(0);
  let rect = $state<DOMRect | null>(null);
  let viewport = $state({ w: window.innerWidth, h: window.innerHeight });

  const step = $derived(steps[index]);
  const last = $derived(index === steps.length - 1);
  const find = (target?: string) => (target ? document.querySelector<HTMLElement>(`[data-tour="${target}"]`) : null);

  function measure() {
    viewport = { w: window.innerWidth, h: window.innerHeight };
    const el = find(step?.target);
    rect = el ? el.getBoundingClientRect() : null;
  }

  $effect(() => {
    void index;
    measure();
  });

  // The spotlight: the target's box with padding, or a point in the middle for intro and outro.
  const spot = $derived(
    rect
      ? { x: rect.left - PAD, y: rect.top - PAD, w: rect.width + PAD * 2, h: rect.height + PAD * 2 }
      : { x: viewport.w / 2, y: viewport.h / 2, w: 0, h: 0 }
  );
  const radius = $derived(rect ? Math.max(spot.w, spot.h) / 2 + 24 : Math.min(viewport.w, viewport.h) * 0.32);

  // The card sits on the roomier side of the target, clamped to the window.
  const card = $derived.by(() => {
    if (!rect) return { left: (viewport.w - CARD_W) / 2, top: viewport.h / 2 - 110, below: false };
    const below = rect.top + rect.height / 2 < viewport.h / 2;
    const left = Math.min(Math.max(16, rect.left + rect.width / 2 - CARD_W / 2), viewport.w - CARD_W - 16);
    return below ? { left, top: rect.bottom + PAD + 18, below } : { left, top: rect.top - PAD - 18, below };
  });

  const go = (to: number) => {
    if (to < 0) return;
    if (to >= steps.length) return onclose();
    index = to;
  };

  function key(e: KeyboardEvent) {
    if (e.key === "Escape") onclose();
    else if (e.key === "ArrowRight" || e.key === "Enter") go(index + 1);
    else if (e.key === "ArrowLeft") go(index - 1);
    else return;
    // The shell's own Esc and F11 handling sits on the same window object.
    e.preventDefault();
    e.stopImmediatePropagation();
  }

  onMount(() => {
    // The app's own pages have a sidebar; YouTube Music's website does not.
    const native = !!find("sidebar");
    steps = ALL_STEPS.filter((s) => (s.native === undefined || s.native === native) && (!s.target || find(s.target)));
    window.addEventListener("resize", measure);
    window.addEventListener("keydown", key, true);
    return () => {
      window.removeEventListener("resize", measure);
      window.removeEventListener("keydown", key, true);
    };
  });
</script>

{#if step}
  <div
    class="tour"
    role="dialog"
    aria-modal="true"
    aria-labelledby="tour-title"
    in:fade={{ duration: 450, easing: cubicOut }}
    out:fade={{ duration: 300 }}
    style:--x="{spot.x + spot.w / 2}px"
    style:--y="{spot.y + spot.h / 2}px"
    style:--r="{radius}px"
  >
    <div class="vignette"></div>
    <div class="spot" class:empty={!rect} style:left="{spot.x}px" style:top="{spot.y}px" style:width="{spot.w}px" style:height="{spot.h}px"></div>

    <div class="card" class:above={rect && !card.below} class:intro={!rect} style:left="{card.left}px" style:top="{card.top}px">
      {#key index}
        <div class="content" in:fade={{ duration: 260, delay: 120 }}>
          {#if !rect}
            <span class="logo"><Icon name={last ? "check" : "play"} size={22} /></span>
          {:else}
            <span class="count">{index} of {steps.length - 2}</span>
          {/if}
          <h2 id="tour-title">{step.title}</h2>
          <p>{step.body}</p>
        </div>
      {/key}
      <div class="footer">
        <div class="dots" aria-hidden="true">
          {#each steps as _, i (i)}<span class:on={i === index}></span>{/each}
        </div>
        {#if !last}<button class="link" onclick={onclose}>Skip</button>{/if}
        {#if index > 0}<button class="btn" onclick={() => go(index - 1)}>Back</button>{/if}
        <button class="btn primary" onclick={() => go(index + 1)}>{index === 0 ? "Show me" : last ? "Start listening" : "Next"}</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .tour { position: fixed; inset: 0; z-index: 1000; }

  /* Heavy vignette centred on the highlighted control: clear around it, near black at the
     window's edges. --x/--y/--r are registered in app.css, so the gradient itself animates. */
  .vignette {
    position: absolute; inset: 0;
    background: radial-gradient(
      circle at var(--x) var(--y),
      rgba(4, 4, 8, 0) 0,
      rgba(4, 4, 8, 0) var(--r),
      rgba(4, 4, 8, 0.6) calc(var(--r) + 140px),
      rgba(4, 4, 8, 0.9) 100%
    );
    transition: --x 0.55s var(--ease), --y 0.55s var(--ease), --r 0.55s var(--ease);
  }

  /* The cut-out: a ring around the control, with an even dim over everything else. */
  .spot {
    position: absolute;
    border-radius: 12px;
    box-shadow:
      0 0 0 1.5px rgba(255, 255, 255, 0.55),
      0 0 28px 6px color-mix(in srgb, var(--accent) 35%, transparent),
      0 0 0 200vmax rgba(4, 4, 8, 0.45);
    transition: left 0.55s var(--ease), top 0.55s var(--ease), width 0.55s var(--ease), height 0.55s var(--ease), box-shadow 0.4s;
    pointer-events: none;
  }
  .spot.empty { box-shadow: 0 0 0 200vmax rgba(4, 4, 8, 0.55); }

  .card {
    position: absolute;
    width: 340px;
    padding: 20px 20px 16px;
    border-radius: var(--radius);
    background: var(--bg-highlight);
    border: 1px solid var(--border);
    box-shadow: 0 24px 64px rgba(0, 0, 0, 0.55);
    transition: left 0.55s var(--ease), top 0.55s var(--ease), transform 0.55s var(--ease);
  }
  .card.above { transform: translateY(-100%); }
  .card.intro { text-align: center; }

  .content { min-height: 92px; }
  .logo {
    width: 44px; height: 44px; margin: 0 auto 14px;
    display: grid; place-items: center;
    border-radius: 12px; background: var(--accent-gradient); color: #fff;
    box-shadow: 0 0 32px color-mix(in srgb, var(--accent) 50%, transparent);
  }
  .count { font-size: 11.5px; font-weight: 600; color: var(--accent); }
  h2 { margin: 4px 0 6px; font-size: 17px; line-height: 1.3; }
  p { margin: 0; color: var(--text-muted); font-size: 13.5px; line-height: 1.5; }

  /* The progress dots take a line of their own above the buttons: with many steps they
     would otherwise push the buttons past the card. */
  .footer { display: flex; flex-wrap: wrap; align-items: center; justify-content: flex-end; gap: 8px; margin-top: 18px; }
  .dots { flex: 1 0 100%; display: flex; gap: 4px; margin-bottom: 6px; min-width: 0; }
  .card.intro .dots { justify-content: center; }
  .footer .link { margin-right: auto; }
  .dots span { flex: 0 1 5px; min-width: 3px; height: 5px; border-radius: 3px; background: rgba(255, 255, 255, 0.18); transition: flex-basis 0.3s var(--ease), background 0.3s; }
  .dots span.on { flex-basis: 14px; background: var(--accent); }
  .link { padding: 6px 8px; color: var(--text-faint); font-size: 13px; font-weight: 600; }
  .link:hover { color: var(--text); }
  .footer .btn { padding: 7px 14px; font-size: 13px; }
</style>
