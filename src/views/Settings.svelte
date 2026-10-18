<script lang="ts">
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { listen } from "@tauri-apps/api/event";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { onMount } from "svelte";
  import Icon from "../components/Icon.svelte";
  import KeybindInput from "../components/KeybindInput.svelte";
  import Row from "../components/Row.svelte";
  import Toggle from "../components/Toggle.svelte";
  import type { IconName } from "../lib/icons";
  import { api, onMemory, onSettings, type Companion, type Memory, type Settings, type ShortcutKey } from "../lib/api";

  type Tab = "general" | "appearance" | "playback" | "audio" | "integrations" | "shortcuts" | "about";
  const tabs: { id: Tab; label: string; icon: IconName }[] = [
    { id: "general", label: "General", icon: "general" },
    { id: "appearance", label: "Appearance", icon: "palette" },
    { id: "playback", label: "Playback", icon: "music" },
    { id: "audio", label: "Audio & lyrics", icon: "equalizer" },
    { id: "integrations", label: "Integrations", icon: "integrations" },
    { id: "shortcuts", label: "Shortcuts", icon: "keyboard" }
  ];
  const shortcutLabels: { key: ShortcutKey; label: string }[] = [
    { key: "playPause", label: "Play / Pause" },
    { key: "next", label: "Next track" },
    { key: "previous", label: "Previous track" },
    { key: "thumbsUp", label: "Thumbs up" },
    { key: "thumbsDown", label: "Thumbs down" },
    { key: "volumeUp", label: "Volume up" },
    { key: "volumeDown", label: "Volume down" }
  ];

  const tabIds: Tab[] = ["general", "appearance", "playback", "audio", "integrations", "shortcuts", "about"];
  const eqBands = ["32", "64", "125", "250", "500", "1K", "2K", "4K", "8K", "16K"];
  const eqPresets: { name: string; gains: number[] }[] = [
    { name: "Flat", gains: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0] },
    { name: "Bass boost", gains: [6, 5, 4, 2, 0, 0, 0, 0, 0, 0] },
    { name: "Treble boost", gains: [0, 0, 0, 0, 0, 1, 2, 4, 5, 6] },
    { name: "Vocal", gains: [-2, -2, -1, 1, 3, 4, 3, 1, 0, -1] },
    { name: "Rock", gains: [4, 3, 2, 0, -1, -1, 1, 2, 3, 4] },
    { name: "Pop", gains: [-1, 1, 3, 4, 3, 0, -1, -1, 0, 1] },
    { name: "Electronic", gains: [5, 4, 1, 0, -2, 1, 0, 2, 4, 5] },
    { name: "Acoustic", gains: [3, 3, 2, 1, 1, 1, 2, 2, 2, 1] }
  ];
  const activePreset = $derived(eqPresets.find((p) => s && p.gains.every((g, i) => g === s!.playback.equalizer.gains[i]))?.name ?? null);
  const requestedTab = new URLSearchParams(location.search).get("tab") as Tab | null;
  let tab = $state<Tab>(requestedTab && tabIds.includes(requestedTab) ? requestedTab : "general");
  let s = $state<Settings | null>(null);
  let memory = $state<Memory | null>(null);
  let companions = $state<Companion[]>([]);
  let lastfmSignedIn = $state(false);
  let info = $state({ version: "", tauri: "" });
  let error = $state<string | null>(null);
  let errorTimer: ReturnType<typeof setTimeout>;

  async function refreshCompanions() {
    companions = await api.companionList();
  }

  onMount(() => {
    api.getSettings().then((v) => (s = v));
    api.getMemory().then((m) => (memory = m));
    api.appInfo().then((i) => (info = i));
    api.lastfmSignedIn().then((v) => (lastfmSignedIn = v));
    refreshCompanions();
    const unlisten = [
      onSettings((v) => (s = v)),
      listen<Tab>("settings-tab", (e) => {
        if (tabIds.includes(e.payload)) tab = e.payload;
      }),
      onMemory((m) => {
        // Authorization window closing usually means a companion was added or denied.
        if (memory?.companionServerAuthWindowEnabled && !m.companionServerAuthWindowEnabled) refreshCompanions();
        memory = m;
        api.lastfmSignedIn().then((v) => (lastfmSignedIn = v));
      })
    ];
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && !(e.target instanceof HTMLButtonElement) && getCurrentWindow().close();
    window.addEventListener("keydown", onKey);
    return () => {
      unlisten.forEach((p) => p.then((f) => f()));
      window.removeEventListener("keydown", onKey);
    };
  });

  function showError(message: unknown) {
    error = String(message);
    clearTimeout(errorTimer);
    errorTimer = setTimeout(() => (error = null), 4000);
  }

  async function set(key: string, value: unknown) {
    try {
      await api.setSetting(key, value);
      s = await api.getSettings();
    } catch (e) {
      showError(e);
      s = await api.getSettings();
    }
  }

  async function pickCss() {
    const path = await api.pickCssFile();
    if (path) await set("appearance.customCssPath", path);
  }

  async function setAuthWindow(enabled: boolean) {
    try {
      await api.companionSetAuthEnabled(enabled);
    } catch (e) {
      showError(e);
    }
  }

  async function revoke(id: string) {
    await api.companionRevoke(id);
    await refreshCompanions();
  }
</script>

<div class="settings">
  <header data-tauri-drag-region>
    <span class="title" data-tauri-drag-region><Icon name="settings" size={18} /> Settings</span>
    <button class="icon-btn" title="Close" onclick={() => getCurrentWindow().close()}><Icon name="close" size={18} /></button>
  </header>

  <div class="body">
    <nav>
      {#each tabs as t (t.id)}
        <button class:active={tab === t.id} onclick={() => (tab = t.id)}><Icon name={t.icon} size={18} />{t.label}</button>
      {/each}
      <span class="push"></span>
      <button class:active={tab === "about"} onclick={() => (tab = "about")}><Icon name="info" size={18} />About</button>
    </nav>

    <main>
      {#if memory?.restartRequired}
        <div class="banner">
          <span><Icon name="refresh" size={18} /> Restart the app to apply changes</span>
          <button class="btn primary" onclick={() => api.restartApp()}>Restart now</button>
        </div>
      {/if}

      {#if s}
        {#key tab}
          <section class="page">
            {#if tab === "general"}
              <h1>General</h1>
              <div class="card">
                <Row title="Hide to tray on close" description="Keep music playing in the background when the window is closed">
                  <Toggle label="Hide to tray on close" checked={s.general.hideToTrayOnClose} onchange={(v) => set("general.hideToTrayOnClose", v)} />
                </Row>
                <Row title="Show notification on song change">
                  <Toggle label="Show notification on song change" checked={s.general.showNotificationOnSongChange} onchange={(v) => set("general.showNotificationOnSongChange", v)} />
                </Row>
                <Row title="Start on boot">
                  <Toggle label="Start on boot" checked={s.general.startOnBoot} onchange={(v) => set("general.startOnBoot", v)} />
                </Row>
                <Row title="Start minimized" description="Start in the tray without showing the window">
                  <Toggle label="Start minimized" checked={s.general.startMinimized} onchange={(v) => set("general.startMinimized", v)} />
                </Row>
                <Row title="Disable hardware acceleration" restart>
                  <Toggle label="Disable hardware acceleration" checked={s.general.disableHardwareAcceleration} onchange={(v) => set("general.disableHardwareAcceleration", v)} />
                </Row>
              </div>
              <h2>Developer</h2>
              <div class="card">
                <Row title="Enable developer tools" description="Adds a developer tools button to the title bar" restart>
                  <Toggle label="Enable developer tools" checked={s.developer.enableDevTools} onchange={(v) => set("developer.enableDevTools", v)} />
                </Row>
              </div>
            {:else if tab === "appearance"}
              <h1>Appearance</h1>
              <div class="card">
                <Row title="Modern UI" description="App player bar with wave seek line and a flat dark YouTube Music theme">
                  <Toggle label="Modern UI" checked={s.appearance.modernUi} onchange={(v) => set("appearance.modernUi", v)} />
                </Row>
                <Row title="Always show volume slider">
                  <Toggle label="Always show volume slider" checked={s.appearance.alwaysShowVolumeSlider} onchange={(v) => set("appearance.alwaysShowVolumeSlider", v)} />
                </Row>
                <Row title="Zoom" description="Scale of the YouTube Music page">
                  <div class="slider">
                    <input type="range" min="50" max="200" step="5" value={s.appearance.zoom} onchange={(e) => set("appearance.zoom", Number(e.currentTarget.value))} />
                    <span class="value">{s.appearance.zoom}%</span>
                  </div>
                </Row>
                <Row title="Tray icon style">
                  <div class="segmented">
                    {#each ["auto", "white", "black"] as style (style)}
                      <button class:active={s.appearance.trayIconStyle === style} onclick={() => set("appearance.trayIconStyle", style)}>{style}</button>
                    {/each}
                  </div>
                </Row>
              </div>
              <h2>Custom CSS</h2>
              <div class="card">
                <Row title="Custom CSS" description="Inject your own stylesheet; it reloads when the file changes">
                  <Toggle label="Custom CSS" checked={s.appearance.customCssEnabled} onchange={(v) => set("appearance.customCssEnabled", v)} />
                </Row>
                {#if s.appearance.customCssEnabled}
                  <Row title="Stylesheet" description={s.appearance.customCssPath ?? "No file selected"} nested>
                    <button class="btn" onclick={pickCss}><Icon name="folder" size={16} />Browse</button>
                  </Row>
                {/if}
              </div>
            {:else if tab === "playback"}
              <h1>Playback</h1>
              <div class="card">
                <Row title="Continue where you left off" description="Restore the last page and track on launch">
                  <Toggle label="Continue where you left off" checked={s.playback.continueWhereYouLeftOff} onchange={(v) => set("playback.continueWhereYouLeftOff", v)} />
                </Row>
                {#if s.playback.continueWhereYouLeftOff}
                  <Row title="Pause on application launch" nested restart>
                    <Toggle label="Pause on application launch" checked={s.playback.continueWhereYouLeftOffPaused} onchange={(v) => set("playback.continueWhereYouLeftOffPaused", v)} />
                  </Row>
                {/if}
                <Row title="Show track progress on taskbar">
                  <Toggle label="Show track progress on taskbar" checked={s.playback.progressInTaskbar} onchange={(v) => set("playback.progressInTaskbar", v)} />
                </Row>
                <Row title="Enable speaker fill" description="Upmix stereo to all available speaker channels" restart>
                  <Toggle label="Enable speaker fill" checked={s.playback.enableSpeakerFill} onchange={(v) => set("playback.enableSpeakerFill", v)} />
                </Row>
                <Row title="Ratio volume" description="Perceptual (cubic) volume curve for finer control at low volume">
                  <Toggle label="Ratio volume" checked={s.playback.ratioVolume} onchange={(v) => set("playback.ratioVolume", v)} />
                </Row>
              </div>
            {:else if tab === "audio"}
              <h1>Audio & lyrics</h1>
              <div class="card">
                <Row title="Show music videos" description="Off plays the audio-only song version where one exists and shows artwork instead of video">
                  <Toggle label="Show music videos" checked={s.playback.showVideos} onchange={(v) => set("playback.showVideos", v)} />
                </Row>
                <Row title="Block ads" description="Removes ads before they load and skips any that still play">
                  <Toggle label="Block ads" checked={s.playback.blockAds} onchange={(v) => set("playback.blockAds", v)} />
                </Row>
                <Row title="Skip disliked songs" description="Moves to the next track as soon as one you have disliked starts">
                  <Toggle label="Skip disliked songs" checked={s.playback.skipDisliked} onchange={(v) => set("playback.skipDisliked", v)} />
                </Row>
                <Row title="Music video quality" description="Preferred resolution when it is available">
                  <div class="segmented">
                    {#each [["auto", "Auto"], ["hd720", "720p"], ["hd1080", "1080p"], ["hd1440", "1440p"], ["hd2160", "4K"]] as [value, label] (value)}
                      <button class:active={s.playback.videoQuality === value} onclick={() => set("playback.videoQuality", value)}>{label}</button>
                    {/each}
                  </div>
                </Row>
                <Row title="Lyric captions" description="Show the current synced lyric line over YouTube Music, like subtitles">
                  <Toggle label="Lyric captions" checked={s.appearance.lyricsCaptions} onchange={(v) => set("appearance.lyricsCaptions", v)} />
                </Row>
              </div>

              <h2>Equalizer</h2>
              <div class="card">
                <Row title="Equalizer" description="10-band equalizer applied to everything YouTube Music plays">
                  <Toggle label="Equalizer" checked={s.playback.equalizer.enabled} onchange={(v) => set("playback.equalizer.enabled", v)} />
                </Row>
                <div class="eq" class:off={!s.playback.equalizer.enabled}>
                  <div class="presets">
                    {#each eqPresets as p (p.name)}
                      <button class:active={activePreset === p.name} onclick={() => set("playback.equalizer.gains", p.gains)}>{p.name}</button>
                    {/each}
                  </div>
                  <div class="bands">
                    {#each eqBands as band, i (band)}
                      <label class="band">
                        <span class="db">{s.playback.equalizer.gains[i] > 0 ? "+" : ""}{s.playback.equalizer.gains[i]}</span>
                        <input
                          type="range"
                          min="-12"
                          max="12"
                          step="1"
                          value={s.playback.equalizer.gains[i]}
                          aria-label="{band} Hz"
                          onchange={(e) => set(`playback.equalizer.gains.${i}`, Number(e.currentTarget.value))}
                        />
                        <span class="hz">{band}</span>
                      </label>
                    {/each}
                  </div>
                </div>
              </div>

              <h2>Playback</h2>
              <div class="card">
                <Row title="Playback speed">
                  <div class="slider">
                    <input type="range" min="50" max="200" step="5" value={s.playback.playbackRate} onchange={(e) => set("playback.playbackRate", Number(e.currentTarget.value))} />
                    <span class="value">{(s.playback.playbackRate / 100).toFixed(2)}×</span>
                  </div>
                </Row>
                <Row title="Fade between tracks" description="Fades the end of each track out and the next one in">
                  <div class="slider">
                    <input type="range" min="0" max="12" step="1" value={s.playback.crossfadeSeconds} onchange={(e) => set("playback.crossfadeSeconds", Number(e.currentTarget.value))} />
                    <span class="value">{s.playback.crossfadeSeconds === 0 ? "Off" : `${s.playback.crossfadeSeconds} s`}</span>
                  </div>
                </Row>
              </div>
            {:else if tab === "integrations"}
              <h1>Integrations</h1>
              <h2>Companion server</h2>
              <div class="card">
                <Row title="Companion server" description="Lets remote controls and stream deck plugins control playback (port 9863)">
                  <Toggle label="Companion server" checked={s.integrations.companionServerEnabled} onchange={(v) => set("integrations.companionServerEnabled", v)} />
                </Row>
                {#if memory?.companionServerError}
                  <div class="alert"><Icon name="warning" size={16} />{memory.companionServerError}</div>
                {/if}
                {#if s.integrations.companionServerEnabled}
                  <Row title="Allow browser communication" description="Any website you visit could communicate with the companion server" danger nested>
                    <Toggle label="Allow browser communication" checked={s.integrations.companionServerCORSWildcardEnabled} onchange={(v) => set("integrations.companionServerCORSWildcardEnabled", v)} />
                  </Row>
                  <Row title="Enable companion authorization" description="Turns off after the first successful authorization or 5 minutes" nested>
                    {#if memory?.companionServerAuthWindowEnabled}<span class="live">Listening</span>{/if}
                    <Toggle label="Enable companion authorization" checked={memory?.companionServerAuthWindowEnabled ?? false} onchange={setAuthWindow} />
                  </Row>
                  <div class="companions">
                    <div class="sub">Authorized companions</div>
                    {#each companions as c (c.id)}
                      <div class="companion">
                        <span class="avatar">{c.appName.slice(0, 1).toUpperCase()}</span>
                        <span class="who"><b>{c.appName}</b><small>{c.appId} · v{c.appVersion}</small></span>
                        <button class="btn danger" onclick={() => revoke(c.id)}><Icon name="delete" size={16} />Revoke</button>
                      </div>
                    {:else}
                      <p class="empty">No companions have been authorized yet.</p>
                    {/each}
                  </div>
                {/if}
              </div>

              <h2>Discord</h2>
              <div class="card">
                <Row title="Discord rich presence" description="Show what you're listening to on your Discord profile">
                  <Toggle label="Discord rich presence" checked={s.integrations.discordPresenceEnabled} onchange={(v) => set("integrations.discordPresenceEnabled", v)} />
                </Row>
                <Row title="Discord application ID" description="Create an application at discord.com/developers; its name is shown as the activity" nested>
                  <input class="text" placeholder="e.g. 1234567890" value={s.integrations.discordClientId} onchange={(e) => set("integrations.discordClientId", e.currentTarget.value.trim())} />
                </Row>
                {#if s.integrations.discordPresenceEnabled && memory?.discordPresenceConnectionFailed}
                  <div class="alert">
                    <Icon name="warning" size={16} />
                    {s.integrations.discordClientId ? "Could not connect to Discord. Is the Discord app running?" : "Enter a Discord application ID."}
                  </div>
                {/if}
              </div>

              <h2>Last.fm</h2>
              <div class="card">
                <Row title="Last.fm scrobbling" description={lastfmSignedIn ? "Connected to Last.fm" : "Approve access in your browser when prompted"}>
                  {#if lastfmSignedIn}
                    <button class="btn" onclick={async () => { await api.lastfmSignOut(); lastfmSignedIn = false; }}>Sign out</button>
                  {/if}
                  <Toggle label="Last.fm scrobbling" checked={s.integrations.lastFMEnabled} onchange={(v) => set("integrations.lastFMEnabled", v)} />
                </Row>
                <Row title="API key" nested>
                  <input class="text" value={s.lastfm.apiKey} onchange={(e) => set("lastfm.apiKey", e.currentTarget.value.trim())} />
                </Row>
                <Row title="Shared secret" description="From last.fm/api/account/create" nested>
                  <input class="text" type="password" value={s.lastfm.secret} onchange={(e) => set("lastfm.secret", e.currentTarget.value.trim())} />
                </Row>
                <Row title="Scrobble percent" description="How much of a track must play before it is scrobbled (max 4 minutes)" nested>
                  <div class="slider">
                    <input type="range" min="1" max="95" value={s.lastfm.scrobblePercent} onchange={(e) => set("lastfm.scrobblePercent", Number(e.currentTarget.value))} />
                    <span class="value">{s.lastfm.scrobblePercent}%</span>
                  </div>
                </Row>
                {#if s.integrations.lastFMEnabled && memory?.lastfmStatus}
                  <div class="alert info"><Icon name="info" size={16} />{memory.lastfmStatus}</div>
                {/if}
                {#if memory && !memory.secureStorageAvailable}
                  <div class="alert"><Icon name="warning" size={16} />The system credential store is unavailable; the Last.fm session can't be saved.</div>
                {/if}
              </div>
            {:else if tab === "shortcuts"}
              <h1>Shortcuts</h1>
              <p class="lead">Global shortcuts work even when the app is in the background. Click a field and press a key combination.</p>
              <div class="card">
                {#each shortcutLabels as sc (sc.key)}
                  <Row title={sc.label}>
                    <KeybindInput value={s.shortcuts[sc.key]} failed={memory?.shortcutRegisterFailed[sc.key] ?? false} onchange={(v) => set(`shortcuts.${sc.key}`, v)} />
                  </Row>
                {/each}
              </div>
              <button class="btn reset" onclick={() => api.resetSettings("shortcuts").then(async () => (s = await api.getSettings()))}>Reset shortcuts</button>
            {:else if tab === "about"}
              <div class="about">
                <div class="hero"><Icon name="play" size={44} /></div>
                <h1>YTMusic Redux</h1>
                <p class="muted">Version {info.version} · Tauri {info.tauri}</p>
                <p class="author">Created by Mirang Bhandari</p>
                <p class="credit">
                  Free and open-source software under the GNU GPL-3.0: you may use, study, share and modify it.
                  Built on the work of
                  <button class="link" onclick={() => openUrl("https://github.com/ytmdesktop/ytmdesktop")}>YouTube Music Desktop App</button>
                  by the YTMDesktop team, with contributions from the community.
                </p>
                <p class="muted small">Not affiliated with Google or YouTube. Updates are not automatic; check the project releases for new versions.</p>
              </div>
            {/if}
          </section>
        {/key}
      {/if}
    </main>
  </div>

  {#if error}
    <div class="toast" role="alert"><Icon name="warning" size={16} />{error}</div>
  {/if}
</div>

<style>
  .settings { height: 100%; display: flex; flex-direction: column; background: var(--bg); border: 1px solid var(--border); }
  header {
    height: 44px; flex: none;
    display: flex; align-items: center; justify-content: space-between;
    padding: 0 6px 0 16px;
    border-bottom: 1px solid var(--border);
  }
  header .title { display: flex; gap: 8px; align-items: center; font-weight: 700; }
  .body { flex: 1; display: flex; min-height: 0; }

  nav {
    width: 210px; flex: none;
    display: flex; flex-direction: column; gap: 2px;
    padding: 12px 10px;
    border-right: 1px solid var(--border);
  }
  nav button {
    position: relative;
    display: flex; align-items: center; gap: 12px;
    padding: 9px 12px;
    border-radius: var(--radius-sm);
    color: var(--text-muted);
    font-weight: 600;
    text-align: left;
    transition: background 0.15s, color 0.15s;
  }
  nav button:hover { background: var(--surface-hover); color: var(--text); }
  nav button.active { background: var(--surface-active); color: var(--text); }
  nav button.active::before {
    content: ""; position: absolute; left: 0; top: 9px; bottom: 9px; width: 3px;
    border-radius: 3px; background: var(--accent);
  }
  nav .push { flex: 1; }

  main { flex: 1; overflow-y: auto; padding: 8px 28px 32px; }
  .page { max-width: 760px; }
  h1 { font-size: 22px; margin: 18px 0 14px; }
  h2 { font-size: 13px; text-transform: uppercase; letter-spacing: 0.8px; color: var(--text-faint); margin: 26px 0 10px; }
  .lead { color: var(--text-muted); margin: -4px 0 16px; }
  .card { background: var(--surface); border: 1px solid var(--border); border-radius: var(--radius); overflow: hidden; }

  .banner {
    position: sticky; top: 0; z-index: 2;
    display: flex; align-items: center; justify-content: space-between;
    margin: 12px 0 0; padding: 10px 12px 10px 16px;
    border-radius: var(--radius);
    background: #24151a; border: 1px solid rgba(240, 68, 95, 0.35);
  }
  .banner span { display: flex; gap: 8px; align-items: center; font-weight: 600; }

  .slider { display: flex; align-items: center; gap: 10px; }
  .slider input { width: 160px; accent-color: var(--accent); }
  .value { min-width: 44px; text-align: right; font-variant-numeric: tabular-nums; color: var(--text-muted); }

  .segmented { display: flex; padding: 3px; gap: 2px; border-radius: 9px; background: var(--surface-hover); }
  .segmented button { padding: 5px 12px; border-radius: 7px; text-transform: capitalize; color: var(--text-muted); font-weight: 600; }
  .segmented button.active { background: var(--accent); color: #fff; }

  input.text {
    width: 240px; height: 34px; padding: 0 10px;
    border-radius: var(--radius-sm);
    background: var(--bg); border: 1px solid var(--border);
    transition: border-color 0.15s;
  }
  input.text:focus { border-color: var(--accent); outline: none; }

  .alert {
    display: flex; gap: 8px; align-items: center;
    margin: 0; padding: 10px 18px;
    color: var(--warning); background: rgba(255, 181, 71, 0.08);
    border-top: 1px solid var(--border); font-size: 13px;
  }
  .alert.info { color: var(--text-muted); background: var(--surface); }

  .live { font-size: 12px; font-weight: 700; color: var(--success); display: flex; align-items: center; gap: 6px; }
  .live::before { content: ""; width: 8px; height: 8px; border-radius: 50%; background: var(--success); }

  .companions { padding: 12px 18px 16px 34px; border-top: 1px solid var(--border); background: rgba(255, 255, 255, 0.015); }
  .sub { font-weight: 600; margin-bottom: 8px; }
  .companion { display: flex; align-items: center; gap: 12px; padding: 8px 0; }
  .avatar {
    width: 34px; height: 34px; border-radius: 10px; flex: none;
    display: grid; place-items: center; font-weight: 800;
    background: var(--surface-active); color: var(--text);
  }
  .who { flex: 1; display: flex; flex-direction: column; min-width: 0; }
  .who small { color: var(--text-faint); }
  .empty { color: var(--text-faint); margin: 4px 0 0; font-size: 13px; }

  .reset { margin-top: 14px; }

  .eq { padding: 4px 18px 18px; border-top: 1px solid var(--border); }
  .eq.off { opacity: 0.45; }
  .presets { display: flex; flex-wrap: wrap; gap: 6px; padding: 12px 0 16px; }
  .presets button {
    padding: 5px 12px; border-radius: 14px;
    border: 1px solid var(--border); color: var(--text-muted);
    font-size: 12.5px; font-weight: 600;
  }
  .presets button:hover { background: var(--surface-hover); color: var(--text); }
  .presets button.active { background: var(--text); border-color: transparent; color: var(--bg); }
  .bands { display: grid; grid-template-columns: repeat(10, 1fr); gap: 4px; }
  .band { display: flex; flex-direction: column; align-items: center; gap: 6px; }
  .band input { writing-mode: vertical-lr; direction: rtl; height: 120px; width: 20px; accent-color: var(--accent); }
  .db { font-size: 11.5px; font-variant-numeric: tabular-nums; color: var(--text-muted); }
  .hz { font-size: 11px; color: var(--text-faint); }

  .about { text-align: center; padding-top: 40px; }
  .hero {
    width: 80px; height: 80px; margin: 0 auto 8px;
    display: grid; place-items: center; border-radius: 20px;
    background: var(--accent); color: #fff;
  }
  .muted { color: var(--text-muted); }
  .small { font-size: 12px; }
  .author { margin: 6px 0 0; font-weight: 600; }
  .credit { max-width: 440px; margin: 20px auto; line-height: 1.6; }
  .link { color: var(--accent); font-weight: 600; padding: 0; }
  .link:hover { text-decoration: underline; }

  .toast {
    position: fixed; bottom: 18px; left: 50%; transform: translateX(-50%);
    display: flex; gap: 8px; align-items: center;
    padding: 10px 16px; border-radius: var(--radius);
    background: #2a1418; border: 1px solid rgba(255, 107, 107, 0.4); color: var(--danger);
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.4);
  }

</style>
