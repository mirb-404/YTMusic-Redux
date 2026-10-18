// YouTube Music bridge, injected into every music.youtube.com document before page scripts run.
//
// Ported from YouTube Music Desktop App (GPL-3.0, https://github.com/ytmdesktop/ytmdesktop):
// src/renderer/ytmview/preload.ts and src/renderer/ytmview/scripts/*.
//
// Data sent from here is untrusted by the Rust side and is normalized there.
(function () {
  if (window.location.hostname !== "music.youtube.com" || window.__YTMR__) return;

  const invoke = (cmd, args) => window.__TAURI_INTERNALS__.invoke(cmd, args || {});
  const playerBar = () => document.querySelector("ytmusic-app-layout>ytmusic-player-bar");

  // --- Hook YTM internals: Polymer instantiates every element through this base class getter.
  window.__YTMD_HOOK__ = {};
  const fakeBaseClass = function () {
    try {
      if (this.hostElement && this.hostElement.nodeName === "YTMUSIC-PLAYER-BAR") {
        window.__YTMD_HOOK__.ytmPlayerBar = this;
      }
      if (this.store && !!this.store.getState && !!this.store.dispatch && !!this.store.subscribe) {
        window.__YTMD_HOOK__.ytmStore = this.store;
      }
    } catch {}
  };
  Object.defineProperty(window, "PolymerFakeBaseClassWithoutHtml", { set: () => {}, get: () => fakeBaseClass });

  const hook = () => window.__YTMD_HOOK__;
  const playerApi = () => hook().ytmPlayerBar.playerApi;
  const ytmStore = () => hook().ytmStore;

  // --- Ad blocking. Ads arrive as slots inside YTM's player and browse responses; they are
  // removed before the page reads them. The hooks are installed here, before page scripts run,
  // and check the setting on every response so it can change without a reload.
  const adblock = {
    enabled: window.__YTMR_ADBLOCK__ === true,
    KEYS: ["adPlacements", "playerAds", "adSlots", "adBreakHeartbeatParams"],
    strip(value) {
      if (!this.enabled || !value || typeof value !== "object") return value;
      for (const target of [value, value.playerResponse]) {
        if (!target || typeof target !== "object") continue;
        for (const key of this.KEYS) if (key in target) delete target[key];
      }
      return value;
    },
    // Ads that slip through are skipped: muted, run to their end, skip button pressed.
    skip() {
      if (!this.enabled) return;
      const video = document.querySelector("#movie_player video");
      if (video && Number.isFinite(video.duration)) {
        video.muted = true;
        video.currentTime = video.duration;
      }
      document.querySelector(".ytp-skip-ad-button, .ytp-ad-skip-button, .ytp-ad-skip-button-modern")?.click();
    }
  };
  const parse = JSON.parse;
  JSON.parse = function (...args) {
    return adblock.strip(parse.apply(this, args));
  };
  const json = Response.prototype.json;
  Response.prototype.json = function () {
    return json.call(this).then((value) => adblock.strip(value));
  };
  let initialResponse;
  Object.defineProperty(window, "ytInitialPlayerResponse", {
    configurable: true,
    get: () => initialResponse,
    set: (value) => (initialResponse = adblock.strip(value))
  });

  const waitFor = (predicate) =>
    new Promise((resolve) => {
      const interval = setInterval(() => {
        try {
          if (predicate()) {
            clearInterval(interval);
            resolve();
          }
        } catch {}
      }, 50);
    });

  const ytAction = (target, actionName, args, returnValue) =>
    target.dispatchEvent(
      new CustomEvent("yt-action", {
        bubbles: true,
        cancelable: false,
        composed: true,
        detail: { actionName, args, optionalAction: false, returnValue: returnValue || [] }
      })
    );

  const navigate = (endpoint) => document.dispatchEvent(new CustomEvent("yt-navigate", { detail: { endpoint } }));

  const toast = (text) =>
    ytAction(document.body, "yt-open-popup-action", [
      {
        openPopupAction: {
          popup: { notificationActionRenderer: { responseText: { runs: [{ text }] } } },
          popupType: "TOAST",
          uniqueId: crypto.randomUUID()
        }
      },
      document.querySelector("ytmusic-app")
    ]);

  const currentLikeStatus = (videoId) => {
    const likeButtonData = playerBar().querySelector("ytmusic-like-button-renderer").data;
    const stored = ytmStore().getState().likeStatus.videos[videoId];
    return stored ? stored : likeButtonData?.likeStatus ?? "UNKNOWN";
  };

  const setVolume = (volume) => {
    playerApi().setVolume(volume);
    ytmStore().dispatch({ type: "SET_VOLUME", payload: volume });
  };

  // --- Styles and injected UI
  function createStyleSheet() {
    const css = document.createElement("style");
    css.textContent = `
      .ytmd-history-back, .ytmd-history-forward { cursor: pointer; margin: 0 18px 0 2px; font-size: 24px; color: rgba(255,255,255,0.5); }
      .ytmd-history-back.pivotbar, .ytmd-history-forward.pivotbar { padding-top: 12px; }
      .ytmd-history-back.disabled, .ytmd-history-forward.disabled { cursor: not-allowed; }
      .ytmd-history-back:hover:not(.disabled), .ytmd-history-forward:hover:not(.disabled) { color: #fff; }
      .ytmd-persist-volume-slider { opacity: 1 !important; pointer-events: initial !important; }
      .ytmd-player-bar-control.library-button, .ytmd-player-bar-control.playlist-button { margin-left: 8px; }
      .ytmd-player-bar-control.library-button.hidden, .ytmd-player-bar-control.playlist-button.hidden { display: none; }
      .ytmd-player-bar-control.sleep-timer-button.active { color: #fff; }
      html[ytmr-hide-video] ytmusic-player { contain: layout; }
      html[ytmr-hide-video] ytmusic-player #song-image img { object-fit: contain !important; }
      html[ytmr-hide-video] ytmusic-player #song-video, html[ytmr-hide-video] ytmusic-player:has(> #ytmr-art[src]) #song-image { visibility: hidden !important; }
      #ytmr-art { display: none; position: absolute; inset: 0; width: 100% !important; height: 100% !important; max-width: none !important; object-fit: contain !important; transform: none !important; border-radius: 0 !important; z-index: 5; pointer-events: none; }
      html[ytmr-hide-video] ytmusic-player #ytmr-art[src] { display: block; }

      /* Page content eases in as YTM renders it, also rows loaded while scrolling. Only
         opacity and transform animate, which the compositor runs off the page's thread. */
      @keyframes ytmr-enter { from { opacity: 0; transform: translateY(14px); } }
      ytmusic-section-list-renderer > #contents > *,
      ytmusic-responsive-header-renderer, ytmusic-immersive-header-renderer, ytmusic-detail-header-renderer,
      ytmusic-editable-playlist-detail-header-renderer, ytmusic-tabbed-search-results-renderer {
        animation: ytmr-enter 0.36s cubic-bezier(0.2, 0.7, 0.2, 1) both;
      }
      ytmusic-section-list-renderer > #contents > :nth-child(2) { animation-delay: 0.04s; }
      ytmusic-section-list-renderer > #contents > :nth-child(3) { animation-delay: 0.08s; }
      ytmusic-section-list-renderer > #contents > :nth-child(n + 4) { animation-delay: 0.12s; }
      ytmusic-two-row-item-renderer .image-wrapper img { transition: transform 0.25s cubic-bezier(0.2, 0.7, 0.2, 1); }
      ytmusic-two-row-item-renderer:hover .image-wrapper img { transform: scale(1.04); }
      yt-page-navigation-progress #progress { background: #f0445f !important; transition: transform 0.2s ease-out !important; }

      #ytmr-up-next {
        position: fixed; right: 24px; bottom: 24px; z-index: 1000; width: 340px; box-sizing: border-box;
        display: grid; grid-template-columns: 56px 1fr auto; align-items: center; gap: 12px; padding: 12px 12px 14px;
        border-radius: 14px; overflow: hidden; color: #fff; font: 13px "Segoe UI", system-ui, sans-serif;
        background: rgba(24, 24, 31, 0.92); border: 1px solid rgba(255,255,255,0.1);
        box-shadow: 0 18px 48px rgba(0,0,0,0.5); backdrop-filter: blur(18px);
        opacity: 0; transform: translateY(18px) scale(0.96); pointer-events: none;
        transition: opacity 0.3s ease, transform 0.4s cubic-bezier(0.2, 0.8, 0.2, 1);
      }
      #ytmr-up-next.show { opacity: 1; transform: none; pointer-events: auto; }
      #ytmr-up-next img { width: 56px; height: 56px; border-radius: 8px; object-fit: cover; background: rgba(255,255,255,0.06); }
      #ytmr-up-next .text { min-width: 0; display: grid; gap: 2px; }
      #ytmr-up-next .label { font-size: 11px; font-weight: 700; letter-spacing: 0.6px; text-transform: uppercase; color: #f0445f; }
      #ytmr-up-next .title { font-weight: 600; font-size: 14px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
      #ytmr-up-next .artist { color: rgba(255,255,255,0.6); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
      #ytmr-up-next .actions { display: flex; gap: 4px; }
      #ytmr-up-next button { display: grid; place-items: center; width: 32px; height: 32px; border: 0; border-radius: 50%; background: rgba(255,255,255,0.08); color: #fff; cursor: pointer; }
      #ytmr-up-next button:hover { background: rgba(255,255,255,0.18); }
      #ytmr-up-next button.play { background: #fff; color: #000; }
      #ytmr-up-next button.play:hover { background: #ddd; }
      #ytmr-up-next svg { width: 18px; height: 18px; fill: currentColor; }
      #ytmr-up-next .countdown { position: absolute; left: 0; bottom: 0; height: 3px; width: 100%; background: linear-gradient(90deg, #f0445f, #ff8fab); transform-origin: left; transition: transform 0.3s linear; }

      @media (prefers-reduced-motion: reduce) {
        ytmusic-section-list-renderer > #contents > *, ytmusic-responsive-header-renderer, ytmusic-immersive-header-renderer,
        ytmusic-detail-header-renderer, ytmusic-editable-playlist-detail-header-renderer, ytmusic-tabbed-search-results-renderer { animation: none; }
        #ytmr-up-next { transition: opacity 0.2s; transform: none; }
      }
    `;
    document.head.appendChild(css);
  }

  function loadMaterialSymbols() {
    return new Promise((resolve) => {
      const link = document.createElement("link");
      link.rel = "stylesheet";
      link.href = "https://fonts.googleapis.com/css2?family=Material+Symbols+Outlined:opsz,wght,FILL,GRAD@24,100,0,0";
      link.onload = link.onerror = () => resolve();
      document.head.appendChild(link);
    });
  }

  function createNavigationArrows() {
    const make = (cls, icon, go) => {
      const el = document.createElement("span");
      el.classList.add("material-symbols-outlined", cls, "disabled");
      el.innerText = icon;
      el.addEventListener("click", () => {
        if (!el.classList.contains("disabled")) go();
      });
      return el;
    };
    const back = make("ytmd-history-back", "west", () => history.back());
    const forward = make("ytmd-history-forward", "east", () => history.forward());

    const refresh = () => {
      // Navigation API (Chromium); other engines fall back to always-enabled arrows.
      const nav = window.navigation;
      back.classList.toggle("disabled", nav ? !nav.canGoBack : false);
      forward.classList.toggle("disabled", nav ? !nav.canGoForward : false);
    };
    if (window.navigation) window.navigation.addEventListener("currententrychange", refresh);
    refresh();

    const pivotBar = document.querySelector("ytmusic-pivot-bar-renderer");
    if (!pivotBar) {
      const searchBar = document.querySelector("ytmusic-search-box");
      searchBar.parentNode.insertBefore(forward, searchBar);
      searchBar.parentNode.insertBefore(back, forward);
    } else {
      forward.classList.add("pivotbar");
      back.classList.add("pivotbar");
      pivotBar.prepend(forward);
      pivotBar.prepend(back);
    }
  }

  // Opens YTM's add-to-playlist dialog for the current track.
  function openAddToPlaylist(anchor, videoId) {
    const returnValue = [];
    ytAction(anchor, "yt-close-popups-action", [["ytmusic-menu-popup-renderer"]]);
    ytAction(anchor, "yt-service-request", [anchor, { addToPlaylistEndpoint: { videoId } }], returnValue);
    returnValue[0].ajaxPromise.then(
      (response) => {
        ytAction(anchor, "yt-open-popup-action", [
          {
            openPopupAction: {
              popup: { addToPlaylistRenderer: response.data.contents[0].addToPlaylistRenderer },
              popupType: "DIALOG"
            }
          },
          anchor
        ]);
      },
      () => {}
    );
  }

  function createAdditionalPlayerBarControls() {
    let currentVideoId = "";
    let libraryFeedbackDefaultToken = "";
    let libraryFeedbackToggledToken = "";
    let sleepTimerTimeout = null;
    const closePopups = (el) => ytAction(el, "yt-close-popups-action", [["ytmusic-menu-popup-renderer"]]);

    const libraryButton = document.createElement("yt-button-shape");
    libraryButton.classList.add("ytmd-player-bar-control", "library-button");
    const libraryButtonData = {
      focused: false,
      iconPosition: "icon-only",
      onTap: function () {
        closePopups(this);
        ytAction(this, "yt-service-request", [
          this,
          { feedbackEndpoint: { feedbackToken: libraryButtonData.toggled ? libraryFeedbackToggledToken : libraryFeedbackDefaultToken } }
        ]);
        ytmStore().dispatch({
          type: "SET_FEEDBACK_TOGGLE_STATE",
          payload: { defaultEndpointFeedbackToken: libraryFeedbackDefaultToken, isToggled: !libraryButtonData.toggled }
        });
      }.bind(libraryButton),
      style: "mono",
      toggled: false,
      toggleable: true,
      type: "text"
    };
    libraryButton.rawProps = { iconName: "yt-sys-icons:library_add", data: libraryButtonData };
    playerBar().querySelector("ytmusic-like-button-renderer").insertAdjacentElement("afterend", libraryButton);

    const playlistButton = document.createElement("yt-button-shape");
    playlistButton.classList.add("ytmd-player-bar-control", "playlist-button");
    playlistButton.rawProps = {
      iconName: "yt-sys-icons:playlist_add",
      data: {
        focused: false,
        iconPosition: "icon-only",
        onTap: function () {
          openAddToPlaylist(this, currentVideoId);
        }.bind(playlistButton),
        style: "mono",
        toggled: false,
        type: "text"
      }
    };
    libraryButton.insertAdjacentElement("afterend", playlistButton);

    playerApi().addEventListener("onVideoDataChange", (event) => {
      if (event.playertype === 1 && (event.type === "dataloaded" || event.type === "dataupdated")) {
        currentVideoId = playerApi().getPlayerResponse().videoDetails.videoId;
      }
    });

    // Sleep timer
    const humanizeTime = (time) => {
      if (time === 1) return `${time} minute`;
      if (time > 1 && time < 60) return `${time} minutes`;
      if (time >= 60 && time < 120) return `${time / 60} hour`;
      return `${time / 60} hours`;
    };
    const sleepItem = (time, iconType, text) => ({
      menuServiceItemRenderer: {
        icon: { iconType },
        serviceEndpoint: { ytmdSleepTimerServiceEndpoint: { time } },
        text: { runs: [{ text }] }
      }
    });
    const sleepTimerButton = document.createElement("yt-icon-button");
    const sleepTimerIcon = document.createElement("yt-icon");
    sleepTimerIcon.set("icon", "TIMER");
    sleepTimerButton.appendChild(sleepTimerIcon);
    sleepTimerButton.setAttribute("title", "Sleep timer off");
    sleepTimerButton.classList.add("ytmusic-player-bar", "ytmd-player-bar-control", "sleep-timer-button");
    sleepTimerButton.onclick = () => {
      const items = [5, 10, 15, 30, 45, 60].map((t) => sleepItem(t, "CLOCK", humanizeTime(t)));
      if (sleepTimerTimeout !== null) items.push(sleepItem(0, "DELETE", "Clear sleep timer"));
      ytAction(sleepTimerButton, "yt-open-popup-action", [
        {
          openPopupAction: {
            popup: { menuPopupRenderer: { accessibilityData: { label: "Action menu" }, items } },
            popupType: "DROPDOWN"
          }
        },
        sleepTimerButton
      ]);
    };
    playerBar().querySelector(".right-controls-buttons .shuffle").insertAdjacentElement("afterend", sleepTimerButton);

    const setSleepTimerIndicator = (time) => {
      sleepTimerButton.classList.toggle("active", time > 0);
      sleepTimerButton.setAttribute("title", time > 0 ? `Sleep timer ${humanizeTime(time)}` : "Sleep timer off");
    };

    window.addEventListener("yt-action", (e) => {
      if (e.detail.actionName !== "yt-service-request") return;
      const endpoint = e.detail.args[1].ytmdSleepTimerServiceEndpoint;
      if (!endpoint) return;
      if (sleepTimerTimeout !== null) {
        clearTimeout(sleepTimerTimeout);
        sleepTimerTimeout = null;
        setSleepTimerIndicator(0);
      }
      if (endpoint.time <= 0) {
        toast("Sleep timer cleared");
        return;
      }
      setSleepTimerIndicator(endpoint.time);
      toast(`Sleep timer set to ${humanizeTime(endpoint.time)}`);
      sleepTimerTimeout = setTimeout(() => {
        sleepTimerTimeout = null;
        setSleepTimerIndicator(0);
        if (playerBar().playing) {
          playerApi().pauseVideo();
          ytAction(document.body, "yt-open-popup-action", [
            {
              openPopupAction: {
                popup: {
                  dismissableDialogRenderer: {
                    title: { runs: [{ text: "Music paused" }] },
                    dialogMessages: [{ runs: [{ text: "Sleep timer expired and your music has been paused" }] }]
                  }
                },
                popupType: "DIALOG"
              }
            },
            document.querySelector("ytmusic-app")
          ]);
        }
      }, endpoint.time * 60 * 1000);
    });

    // Keep the library button in sync with the current track's menu
    ytmStore().subscribe(() => {
      const state = ytmStore().getState();
      const currentMenu = playerBar().getMenuRenderer();
      if (!currentMenu) {
        libraryButton.classList.add("hidden");
        playlistButton.classList.add("hidden");
        return;
      }
      playlistButton.classList.remove("hidden");
      const item = currentMenu.items.find((i) => {
        const icon = i.toggleMenuServiceItemRenderer?.defaultIcon.iconType;
        return icon === "BOOKMARK_BORDER" || icon === "BOOKMARK";
      });
      libraryButton.classList.toggle("hidden", !item);
      if (!item) return;
      const renderer = item.toggleMenuServiceItemRenderer;
      libraryFeedbackDefaultToken = renderer.defaultServiceEndpoint.feedbackEndpoint.feedbackToken;
      libraryFeedbackToggledToken = renderer.toggledServiceEndpoint.feedbackEndpoint.feedbackToken;
      const toggled = state.toggleStates.feedbackToggleStates[libraryFeedbackDefaultToken];
      libraryButtonData.toggled = toggled !== undefined && toggled !== null ? toggled : false;
      libraryButton.setters.data(libraryButtonData);
      const saved = renderer.defaultIcon.iconType === "BOOKMARK_BORDER" ? libraryButtonData.toggled : !libraryButtonData.toggled;
      libraryButton.setters.iconName(saved ? "yt-sys-icons:library_saved" : "yt-sys-icons:library_add");
    });
  }

  function hookPlayerApiEvents() {
    // YTM dispatches store actions constantly (dozens per page navigation). Forwarding each one
    // serialized the whole queue on the page's thread and sent it to Rust, which slowed every
    // navigation. The store is immutable, so unchanged slices keep their identity: only real
    // changes are forwarded.
    let sent = {};
    const sendStoreState = () => {
      const state = ytmStore().getState();
      if (state.player.adPlaying) adblock.skip();
      const videoId = playerApi().getPlayerResponse()?.videoDetails?.videoId;
      const likeStatus = currentLikeStatus(videoId);
      const next = {
        queue: state.queue ?? null,
        likeStatus,
        volume: state.player.volume,
        muted: !!state.player.muted,
        adPlaying: !!state.player.adPlaying
      };
      if (Object.keys(next).every((key) => next[key] === sent[key])) return;
      sent = next;
      audio.skipIfDisliked(videoId, likeStatus);
      // Only the fields needed to operate are forwarded; the store contains sensitive data.
      invoke("ytm_store_state", next);
    };

    playerApi().addEventListener("onVideoProgress", (progress) => invoke("ytm_video_progress", { progress }));
    playerApi().addEventListener("onStateChange", (state) => invoke("ytm_video_state", { state }));
    playerApi().addEventListener("onVideoDataChange", (event) => {
      if (event.playertype !== 1 || (event.type !== "dataloaded" && event.type !== "dataupdated")) return;
      const videoDetails = playerApi().getPlayerResponse().videoDetails;
      audio.onTrack(videoDetails.videoId);
      let album = null;
      let hasFullMetadata = false;
      const currentItem = playerBar().currentItem;
      if (currentItem !== null && currentItem !== undefined) {
        hasFullMetadata = true;
        videoDetails.title = currentItem.title.runs.map((v) => v.text).join("");
        videoDetails.thumbnail = currentItem.thumbnail;
        for (const run of currentItem.longBylineText.runs) {
          const pageType =
            run.navigationEndpoint?.browseEndpoint?.browseEndpointContextSupportedConfigs?.browseEndpointContextMusicConfig?.pageType;
          if (pageType === "MUSIC_PAGE_TYPE_ALBUM") {
            album = { id: run.navigationEndpoint.browseEndpoint.browseId, text: run.text };
          }
        }
      }
      invoke("ytm_video_data", {
        videoDetails,
        playlistId: playerApi().getPlaylistId() ?? null,
        album,
        likeStatus: currentLikeStatus(videoDetails.videoId),
        hasFullMetadata
      });
    });
    ytmStore().subscribe(sendStoreState);

    window.addEventListener("yt-action", (e) => {
      if (e.detail.actionName === "yt-service-request") {
        if (e.detail.args[1].createPlaylistServiceEndpoint) {
          const title = e.detail.args[2].create_playlist_title;
          e.detail.returnValue[0].ajaxPromise.then((response) =>
            invoke("ytm_playlist_created", { playlist: { id: response.data.playlistId, title } })
          );
        }
      } else if (e.detail.actionName === "yt-handle-playlist-deletion-command") {
        invoke("ytm_playlist_deleted", { playlistId: e.detail.args[0].handlePlaylistDeletionCommand.playlistId });
      }
    });

    document.addEventListener("yt-navigate-finish", () => invoke("ytm_url_changed", { url: location.href }));
    document.addEventListener("fullscreenchange", () => invoke("ytm_fullscreen", { fullscreen: !!document.fullscreenElement }));
  }

  function continueWhereYouLeftOff(config) {
    if (!config.continueWhereYouLeftOff) return;
    if (config.lastUrl.startsWith("https://music.youtube.com/watch")) {
      const response = playerApi().getPlayerResponse();
      if (response) {
        invoke("ytm_video_data", {
          videoDetails: response.videoDetails,
          playlistId: playerApi().getPlaylistId() ?? null,
          album: null,
          likeStatus: null,
          hasFullMetadata: false
        });
      }
      return;
    }
    if (!config.lastVideoId) return;
    // Height transition hack keeps the "Start playback" hint positioned correctly (upstream #1159).
    let transitions = 0;
    const onTransitionEnd = (e) => {
      if (e.target === playerBar() && e.propertyName === "height") {
        document.querySelector("ytmusic-popup-container").refitPopups_?.();
        if (++transitions >= 2) playerBar().removeEventListener("transitionend", onTransitionEnd);
      }
    };
    playerBar().addEventListener("transitionend", onTransitionEnd);
    navigate({ watchEndpoint: { videoId: config.lastVideoId, playlistId: config.lastPlaylistId } });
  }

  // --- Ratio volume (based on https://greasyfork.org/en/scripts/397686 by Marco Pfeiffer)
  const ratioVolume = {
    original: null,
    enable() {
      if (this.original) return;
      const EXPONENT = 3;
      const stored = new WeakMap();
      const original = Object.getOwnPropertyDescriptor(HTMLMediaElement.prototype, "volume");
      this.original = original;
      Object.defineProperty(HTMLMediaElement.prototype, "volume", {
        configurable: true,
        get() {
          const calculated = original.get.call(this) ** (1 / EXPONENT);
          const storedVolume = stored.get(this);
          return Math.abs(storedVolume - calculated) < 0.01 ? storedVolume : calculated;
        },
        set(volume) {
          stored.set(this, volume);
          original.set.call(this, volume ** EXPONENT);
        }
      });
      setVolume(playerApi().getVolume());
    },
    disable() {
      if (!this.original) return;
      Object.defineProperty(HTMLMediaElement.prototype, "volume", this.original);
      this.original = null;
      setVolume(playerApi().getVolume());
    }
  };

  // --- Modern UI theme (the app renders the player bar; YTM's is hidden by the theme)
  function setModernUi(css) {
    let el = document.getElementById("ytmr-modern-ui");
    if (css === null) {
      el?.remove();
      return;
    }
    if (!el) {
      el = document.createElement("style");
      el.id = "ytmr-modern-ui";
      (document.head || document.documentElement).appendChild(el);
    }
    el.textContent = css;
  }


  // --- Audio: song/video preference, playback speed, equalizer and track fades
  const audio = {
    config: null,
    graph: null,
    trackId: "",
    manualSwitch: false,
    watched: null,
    media: () => document.querySelector("#movie_player video") || document.querySelector("video"),
    skippedId: "",
    skipIfDisliked(videoId, status) {
      if (!this.config?.skipDisliked || status !== "DISLIKE" || !videoId || this.skippedId === videoId) return;
      this.skippedId = videoId;
      playerApi().nextVideo();
    },
    // YTM's player reads its quality preference from localStorage when a video loads.
    applyQuality() {
      const quality = this.config.videoQuality;
      if (!quality || quality === "auto") return localStorage.removeItem("yt-player-quality");
      const now = Date.now();
      localStorage.setItem("yt-player-quality", JSON.stringify({ data: quality, expiration: now + 30 * 86400000, creation: now }));
    },
    set(config) {
      this.config = config;
      this.applyQuality();
      this.applyRate();
      this.applyGraph();
      this.applyVideoMode();
    },
    onTrack(videoId) {
      if (!this.config || videoId === this.trackId) return;
      this.trackId = videoId;
      this.applyRate();
      // A user's own click on YTM's Song/Video switch wins for that track.
      if (this.manualSwitch) {
        this.manualSwitch = false;
        setTimeout(() => this.applyArt(), 400);
        return;
      }
      setTimeout(() => this.applyVideoMode(), 400);
    },
    applyRate() {
      const el = this.media();
      const rate = this.config.playbackRate / 100;
      if (el && el.playbackRate !== rate) el.playbackRate = rate;
    },
    applyVideoMode() {
      const player = document.querySelector("ytmusic-player");
      if (!player) return;
      const show = this.config.showVideos;
      // YTM's own switch changes the stream, so audio-only also saves bandwidth.
      const toggle = document.querySelector("ytmusic-av-toggle");
      const wanted = show ? "OMV_PREFERRED" : "ATV_PREFERRED";
      if (toggle && !toggle.hasAttribute("toggle-disabled") && player.getAttribute("playback-mode") !== wanted) {
        toggle.querySelector(show ? ".video-button" : ".song-button")?.click();
      }
      this.applyArt();
      document.documentElement.toggleAttribute("ytmr-hide-video", !show);
      this.watchPlayer(player);
    },
    // YTM re-renders the player (opening its page, fullscreen), which can drop the art.
    watchPlayer(player) {
      if (this.watched === player) return;
      this.watched = player;
      new MutationObserver(() => {
        const img = document.getElementById("ytmr-art");
        if (!img || img.parentNode !== player) this.applyArt();
      }).observe(player, { childList: true, attributes: true, attributeFilter: ["player-ui-state"] });
    },
    // With videos off the player area shows the track's artwork, also for tracks that only
    // exist as a video. The queue item's thumbnail replaces the response's one on track
    // change; it is small, so googleusercontent URLs are asked for at full size.
    applyArt() {
      const player = document.querySelector("ytmusic-player");
      if (!player) return;
      const thumbnails = hook().ytmPlayerBar?.playerApi?.getPlayerResponse()?.videoDetails?.thumbnail?.thumbnails;
      const largest = thumbnails?.length ? [...thumbnails].sort((a, b) => b.width - a.width)[0] : null;
      // googleusercontent URLs end in sizing options and are served at exactly that size, so
      // the larger size keeps the thumbnail's own aspect ratio: a fixed square stretched
      // every non-square (video) thumbnail.
      let art = largest?.url ?? null;
      if (art && largest.width > 0 && largest.height > 0) {
        const scale = Math.max(1, 1200 / Math.max(largest.width, largest.height));
        const w = Math.round(largest.width * scale);
        const h = Math.round(largest.height * scale);
        art = art.replace(/=w\d+-h\d+[^/]*$/, `=w${w}-h${h}-l90-rj`);
      }
      let img = document.getElementById("ytmr-art");
      if (!img || img.parentNode !== player) {
        img?.remove();
        img = document.createElement("img");
        img.id = "ytmr-art";
        img.alt = "";
        player.appendChild(img);
      }
      if (art) img.src = art;
      else img.removeAttribute("src");
    },
    // Media can be routed through Web Audio only once, so the graph is created on first
    // need and kept; neutral settings make it a pass-through.
    ensureGraph() {
      const el = this.media();
      if (!el) return null;
      if (this.graph && this.graph.el === el) return this.graph;
      this.graph?.ctx.close();
      const ctx = new AudioContext();
      const source = ctx.createMediaElementSource(el);
      const bands = this.config.eqBands;
      const filters = bands.map((frequency, i) => {
        const f = ctx.createBiquadFilter();
        f.type = i === 0 ? "lowshelf" : i === bands.length - 1 ? "highshelf" : "peaking";
        f.frequency.value = frequency;
        f.Q.value = 1.1;
        return f;
      });
      const fade = ctx.createGain();
      filters.reduce((prev, f) => (prev.connect(f), f), source).connect(fade);
      fade.connect(ctx.destination);
      el.addEventListener("play", () => ctx.resume());
      el.addEventListener("playing", () => this.applyRate());
      el.addEventListener("timeupdate", () => this.updateFade());
      ctx.resume();
      this.graph = { ctx, el, filters, fade };
      return this.graph;
    },
    applyGraph() {
      const { equalizer, crossfadeSeconds } = this.config;
      if (!this.graph && !equalizer.enabled && crossfadeSeconds === 0) return;
      const g = this.ensureGraph();
      if (!g) return;
      g.filters.forEach((f, i) => f.gain.setTargetAtTime(equalizer.enabled ? equalizer.gains[i] : 0, g.ctx.currentTime, 0.05));
      this.updateFade();
    },
    updateFade() {
      const g = this.graph;
      if (!g) return;
      const seconds = this.config.crossfadeSeconds;
      const { currentTime: t, duration: d, playbackRate: rate } = g.el;
      let level = 1;
      // Skipped for live streams and tracks too short to fade both ends.
      if (seconds > 0 && Number.isFinite(d) && d >= seconds * 3) {
        // timeupdate fires about every 250 ms, so the level aims at the next update's position:
        // the fade reaches silence as the track ends instead of lagging past it.
        const ahead = t + 0.25 * rate;
        const x = Math.max(0, Math.min(1, ahead / seconds, (d - ahead) / seconds));
        // Squared, so loudness falls evenly instead of holding and then dropping at the end.
        level = x * x;
      }
      g.fade.gain.setTargetAtTime(level, g.ctx.currentTime, 0.06);
    }
  };
  // --- Up next: a card announcing the next track during the last seconds of the current one.
  const UP_NEXT_SECONDS = 15;
  const upNext = {
    el: null,
    shownFor: "",
    dismissed: "",
    // The queue entry after the playing one, else the first autoplay suggestion.
    next() {
      const queue = ytmStore().getState().queue;
      if (!queue || queue.repeatMode === "ONE") return null;
      const renderer = (item) =>
        item?.playlistPanelVideoRenderer || item?.playlistPanelVideoWrapperRenderer?.primaryRenderer?.playlistPanelVideoRenderer;
      const items = queue.items || [];
      const i = items.findIndex((item) => renderer(item)?.selected);
      if (i < 0) return null;
      return renderer(items[i + 1]) || renderer(queue.automixItems?.[0]) || (queue.repeatMode === "ALL" ? renderer(items[0]) : null);
    },
    build() {
      const el = document.createElement("div");
      el.id = "ytmr-up-next";
      const img = document.createElement("img");
      img.alt = "";
      const text = document.createElement("div");
      text.className = "text";
      for (const cls of ["label", "title", "artist"]) {
        const span = document.createElement("span");
        span.className = cls;
        text.appendChild(span);
      }
      const actions = document.createElement("div");
      actions.className = "actions";
      const button = (cls, title, icon, onclick) => {
        const b = document.createElement("button");
        b.className = cls;
        b.title = title;
        b.appendChild(svgIcon(icon, 18));
        b.addEventListener("click", onclick);
        return b;
      };
      actions.append(
        button("play", "Play now", "skipNext", () => {
          this.hide();
          playerApi().nextVideo();
        }),
        button("", "Dismiss", "close", () => {
          this.dismissed = playerApi().getPlayerResponse()?.videoDetails?.videoId ?? "";
          this.hide();
        })
      );
      const countdown = document.createElement("span");
      countdown.className = "countdown";
      el.append(img, text, actions, countdown);
      document.body.appendChild(el);
      this.el = el;
    },
    hide() {
      this.el?.classList.remove("show");
    },
    update(media) {
      const { currentTime: t, duration: d, playbackRate: rate } = media;
      const id = playerApi().getPlayerResponse()?.videoDetails?.videoId ?? "";
      const left = (d - t) / (rate || 1);
      const due = Number.isFinite(d) && d > UP_NEXT_SECONDS * 3 && left > 0.5 && left <= UP_NEXT_SECONDS;
      if (!due || this.dismissed === id || ytmStore().getState().player.adPlaying) return this.hide();
      const next = this.next();
      if (!next) return this.hide();
      if (!this.el) this.build();
      const key = id + next.videoId;
      if (this.shownFor !== key) {
        this.shownFor = key;
        const thumbs = next.thumbnail?.thumbnails ?? [];
        const [img, text] = this.el.children;
        img.src = thumbs[thumbs.length - 1]?.url ?? "";
        const runs = (v) => v?.runs?.map((r) => r.text).join("") ?? "";
        text.children[1].textContent = runs(next.title);
        text.children[2].textContent = runs(next.shortBylineText);
      }
      this.el.children[1].children[0].textContent = `Up next · ${Math.ceil(left)}s`;
      this.el.lastChild.style.transform = `scaleX(${left / UP_NEXT_SECONDS})`;
      this.el.classList.add("show");
    }
  };
  // Media events do not bubble, so they are caught on the way down.
  document.addEventListener(
    "timeupdate",
    (e) => {
      if (e.target !== audio.media()) return;
      try {
        upNext.update(e.target);
      } catch {
        upNext.hide();
      }
    },
    true
  );

  document.addEventListener(
    "click",
    (e) => {
      if (e.isTrusted && e.target.closest?.("ytmusic-av-toggle")) audio.manualSwitch = true;
    },
    true
  );

  // --- Keyboard controls: the app's key map (keyboard.rs), matched as the shell matches it
  // (src/lib/keyboard.ts). Caught before YTM's own handlers, so a key never acts twice.
  const REPEATING_KEYS = new Set(["volumeUp", "volumeDown", "seekForward", "seekBack"]);
  const keyCombo = (e) => {
    const key = e.key === " " ? "Space" : e.key.length === 1 ? e.key.toUpperCase() : e.key;
    const shift = e.shiftKey && (e.key.length > 1 || /^[a-z]$/i.test(e.key));
    return [e.ctrlKey && "Ctrl", e.altKey && "Alt", shift && "Shift", key].filter(Boolean).join("+");
  };
  const keyIgnored = (e, keys) => {
    if (e.target.closest?.("input:not([type=range]):not([type=checkbox]), textarea, select, [contenteditable], ytmusic-search-box")) {
      return !(e.ctrlKey || e.altKey || /^F\d+$/.test(e.key));
    }
    return keys === "Space" && !!e.target.closest?.("button, a, input, [role=button], [role=slider], tp-yt-paper-item, tp-yt-paper-icon-button, yt-button-shape");
  };
  waitFor(() => window.__TAURI_INTERNALS__)
    .then(() => invoke("keyboard_bindings"))
    .then((bindings) => {
      const actions = new Map(bindings.flatMap((b) => b.keys.map((k) => [k, b.action])));
      window.addEventListener(
        "keydown",
        (e) => {
          if (e.isComposing) return;
          const keys = keyCombo(e);
          const action = actions.get(keys);
          if (!action || keyIgnored(e, keys)) return;
          e.preventDefault();
          e.stopImmediatePropagation();
          if (e.repeat && !REPEATING_KEYS.has(action)) return;
          invoke("keyboard_action", { action });
        },
        true
      );
    });
  // Esc leaves app fullscreen too, unless YTM used it (closing a dialog or the search box)
  // or the page itself is fullscreen, which the browser exits on its own.
  let appFullscreen = false;
  window.addEventListener("keydown", (e) => {
    if (e.key !== "Escape" || !appFullscreen || e.defaultPrevented || document.fullscreenElement) return;
    if (e.target.closest?.("input, textarea, [contenteditable], tp-yt-paper-dialog, tp-yt-iron-dropdown")) return;
    invoke("ytm_toggle_fullscreen");
  });

  // --- Window controls in YTM's header, which replaces the app title bar (one header).
  const ICONS = {
    plugins: "M20.5 11H19V7c0-1.1-.9-2-2-2h-4V3.5C13 2.12 11.88 1 10.5 1S8 2.12 8 3.5V5H4c-1.1 0-1.99.9-1.99 2v3.8H3.5c1.49 0 2.7 1.21 2.7 2.7s-1.21 2.7-2.7 2.7H2V20c0 1.1.9 2 2 2h3.8v-1.5c0-1.49 1.21-2.7 2.7-2.7 1.49 0 2.7 1.21 2.7 2.7V22H17c1.1 0 2-.9 2-2v-4h1.5c1.38 0 2.5-1.12 2.5-2.5S21.88 11 20.5 11z",
    mini: "M19 11h-8v6h8v-6zm4 8V4.98C23 3.88 22.1 3 21 3H3c-1.1 0-2 .88-2 1.98V19c0 1.1.9 2 2 2h18c1.1 0 2-.9 2-2zm-2 .02H3V4.97h18v14.05z",
    settings: "M19.14 12.94c.04-.3.06-.61.06-.94 0-.32-.02-.64-.07-.94l2.03-1.58a.49.49 0 0 0 .12-.61l-1.92-3.32a.488.488 0 0 0-.59-.22l-2.39.96c-.5-.38-1.03-.7-1.62-.94l-.36-2.54a.484.484 0 0 0-.48-.41h-3.84c-.24 0-.43.17-.47.41l-.36 2.54c-.59.24-1.13.57-1.62.94l-2.39-.96c-.22-.08-.47 0-.59.22L2.74 8.87c-.12.21-.08.47.12.61l2.03 1.58c-.05.3-.09.63-.09.94s.02.64.07.94l-2.03 1.58a.49.49 0 0 0-.12.61l1.92 3.32c.12.22.37.29.59.22l2.39-.96c.5.38 1.03.7 1.62.94l.36 2.54c.05.24.24.41.48.41h3.84c.24 0 .44-.17.47-.41l.36-2.54c.59-.24 1.13-.56 1.62-.94l2.39.96c.22.08.47 0 .59-.22l1.92-3.32c.12-.22.07-.47-.12-.61l-2.01-1.58zM12 15.6c-1.98 0-3.6-1.62-3.6-3.6s1.62-3.6 3.6-3.6 3.6 1.62 3.6 3.6-1.62 3.6-3.6 3.6z",
    skipNext: "M6 18l8.5-6L6 6v12zM16 6v12h2V6h-2z",
    minimize: "M5 11h14v2H5z",
    maximize: "M4 4h16v16H4V4zm2 2v12h12V6H6z",
    restore: "M8 4h12v12h-2V6H8V4zM4 8h12v12H4V8zm2 2v8h8v-8H6z",
    fullscreenExit: "M5 16h3v3h2v-5H5v2zm3-8H5v2h5V5H8v3zm6 11h2v-3h3v-2h-5v5zm2-11V5h-2v5h5V8h-3z",
    close: "M19 6.41L17.59 5 12 10.59 6.41 5 5 6.41 10.59 12 5 17.59 6.41 19 12 13.41 17.59 19 19 17.59 13.41 12z"
  };
  // Clicks on these start no window drag.
  const NO_DRAG = "a, button, input, ytmusic-search-box, #right-content, #guide-button, tp-yt-paper-icon-button, yt-icon-button, .ytmd-history-back, .ytmd-history-forward";
  const windowAction = (action) => invoke("ytm_window_action", { action });
  // YTM enforces Trusted Types, so markup is built with DOM calls instead of innerHTML.
  const svgIcon = (icon, size) => {
    const ns = "http://www.w3.org/2000/svg";
    const svg = document.createElementNS(ns, "svg");
    svg.setAttribute("width", size);
    svg.setAttribute("height", size);
    svg.setAttribute("viewBox", "0 0 24 24");
    const path = document.createElementNS(ns, "path");
    path.setAttribute("d", ICONS[icon]);
    svg.appendChild(path);
    return svg;
  };

  function createWindowControls() {
    const css = document.createElement("style");
    css.textContent = `
      #ytmr-window-controls { position: fixed; top: 0; right: 0; z-index: 1001; display: flex; align-items: flex-start; }
      #ytmr-window-controls button { display: grid; place-items: center; border: 0; background: none; color: rgba(255,255,255,0.72); cursor: pointer; }
      #ytmr-window-controls button:hover { background: rgba(255,255,255,0.08); color: #fff; }
      #ytmr-window-controls svg { fill: currentColor; }
      #ytmr-window-controls .app { width: 36px; height: 36px; margin-top: 14px; border-radius: 8px; }
      #ytmr-window-controls .plugins { width: auto; padding: 0 10px; gap: 6px; display: flex; align-items: center; font: 600 13px "Segoe UI", system-ui, sans-serif; }
      #ytmr-window-controls .divider { width: 1px; height: 18px; margin: 23px 8px 0; background: rgba(255,255,255,0.12); }
      #ytmr-window-controls .win { width: 46px; height: 36px; }
      #ytmr-window-controls .win.close:hover { background: #c42b1c; }
      #ytmr-window-controls .fs, #ytmr-window-controls.fullscreen .win:not(.close):not(.fs) { display: none; }
      #ytmr-window-controls.fullscreen .fs { display: grid; }
      ytmusic-nav-bar #right-content { transform: translateX(calc(-1 * var(--ytmr-controls-width, 0px))); }
    `;
    document.head.appendChild(css);

    const bar = document.createElement("div");
    bar.id = "ytmr-window-controls";
    const button = (cls, title, icon, size, action, label) => {
      const b = document.createElement("button");
      b.className = cls;
      b.title = title;
      b.append(svgIcon(icon, size), label || "");
      b.addEventListener("click", () => windowAction(action));
      return b;
    };
    const divider = document.createElement("span");
    divider.className = "divider";
    const maximize = button("win", "Maximize", "maximize", 13, "toggleMaximize");
    const exitFullscreen = button("win fs", "Exit full screen (F11)", "fullscreenExit", 18, "toggleFullscreen");
    bar.append(
      button("app plugins", "Plugins", "plugins", 16, "plugins", "Plugins"),
      button("app", "Mini player", "mini", 18, "mini"),
      button("app", "Settings", "settings", 18, "settings"),
      divider,
      button("win", "Minimize", "minimize", 14, "minimize"),
      maximize,
      exitFullscreen,
      button("win close", "Close", "close", 16, "close")
    );
    document.body.appendChild(bar);
    document.documentElement.style.setProperty("--ytmr-controls-width", `${bar.offsetWidth}px`);

    window.__YTMR__.setFullscreen = (on) => {
      appFullscreen = on;
      bar.classList.toggle("fullscreen", on);
    };
    window.__YTMR__.setMaximized = (maximized) => {
      maximize.title = maximized ? "Restore" : "Maximize";
      maximize.querySelector("path").setAttribute("d", ICONS[maximized ? "restore" : "maximize"]);
    };

    // The empty parts of the header move the window; double click maximizes.
    document.querySelector("ytmusic-nav-bar").addEventListener("mousedown", (e) => {
      // A fullscreen window has nothing to drag or maximize.
      if (e.button !== 0 || appFullscreen || e.target.closest(NO_DRAG)) return;
      windowAction(e.detail === 2 ? "toggleMaximize" : "drag");
    });
  }

  // --- YTM's "Saved to Liked music" style toasts stay until dismissed; close them after 3 s.
  function autoCloseToasts() {
    const armed = new WeakSet();
    const scan = () => {
      for (const toast of document.querySelectorAll("tp-yt-paper-toast.paper-toast-open")) {
        if (armed.has(toast)) continue;
        armed.add(toast);
        setTimeout(() => {
          armed.delete(toast);
          if (!toast.classList.contains("paper-toast-open")) return;
          const close = toast.querySelector("#close-button button");
          if (close) close.click();
          else toast.close?.();
        }, 3000);
      }
    };
    new MutationObserver(scan).observe(document.body, { childList: true, subtree: true, attributes: true, attributeFilter: ["class"] });
  }

  // --- Custom CSS
  function setCustomCss(css) {
    let el = document.getElementById("ytmr-custom-css");
    if (css === null) {
      el?.remove();
      return;
    }
    if (!el) {
      el = document.createElement("style");
      el.id = "ytmr-custom-css";
      document.head.appendChild(el);
    }
    el.textContent = css;
  }

  function setVolumeSliderPersist(persist) {
    document.querySelector("ytmusic-app-layout>ytmusic-player-bar #volume-slider")?.classList.toggle("ytmd-persist-volume-slider", persist);
  }

  function toggleRating(target) {
    const videoId = playerApi().getPlayerResponse().videoDetails.videoId;
    const button = document.querySelector("ytmusic-like-button-renderer");
    const endpoints = {};
    for (const endpoint of playerBar().querySelector("ytmusic-like-button-renderer").data.serviceEndpoints) {
      endpoints[endpoint.likeEndpoint.status] = endpoint;
    }
    const status = currentLikeStatus(videoId);
    let endpoint = null;
    if (status === target) endpoint = endpoints.INDIFFERENT;
    else if (status === "INDIFFERENT" || status === "LIKE" || status === "DISLIKE") endpoint = endpoints[target];
    if (endpoint) ytAction(button, "yt-service-request", [button, endpoint]);
  }

  async function getPlaylists(requestId) {
    let playlists = [];
    try {
      const returnValue = [];
      ytAction(playerBar(), "yt-service-request", [playerBar(), { addToPlaylistEndpoint: { videoId: playerApi().getPlayerResponse().videoDetails.videoId } }], returnValue);
      const response = await returnValue[0].ajaxPromise;
      playlists = response.data.contents[0].addToPlaylistRenderer.playlists.map((p) => ({
        id: p.playlistAddToOptionRenderer.playlistId,
        title: p.playlistAddToOptionRenderer.title.runs.map((r) => r.text).join("")
      }));
    } catch {}
    invoke("ytm_playlists_response", { requestId, playlists });
  }

  function exec(command, value) {
    const api = playerApi();
    switch (command) {
      case "playPause":
        playerBar().playing ? api.pauseVideo() : api.playVideo();
        break;
      case "play":
        api.playVideo();
        break;
      case "pause":
        api.pauseVideo();
        break;
      case "next":
        api.nextVideo();
        break;
      case "previous":
        api.previousVideo();
        break;
      case "toggleLike":
        toggleRating("LIKE");
        break;
      case "toggleDislike":
        toggleRating("DISLIKE");
        break;
      case "volumeUp":
        setVolume(Math.min(api.getVolume() + 10, 100));
        break;
      case "volumeDown":
        setVolume(Math.max(api.getVolume() - 10, 0));
        break;
      case "setVolume":
        setVolume(value);
        break;
      case "mute":
        api.mute();
        ytmStore().dispatch({ type: "SET_MUTED", payload: true });
        break;
      case "unmute":
        api.unMute();
        ytmStore().dispatch({ type: "SET_MUTED", payload: false });
        break;
      case "repeatMode":
        ytmStore().dispatch({ type: "SET_REPEAT", payload: value });
        break;
      case "seekTo":
        api.seekTo(value);
        break;
      case "shuffle":
        playerBar().queue.shuffle();
        break;
      case "playQueueIndex": {
        const queue = ytmStore().getState().queue;
        let index = value;
        let list = queue.items;
        if (index > queue.items.length - 1) {
          index -= queue.items.length;
          list = queue.automixItems;
        }
        const song = list[index];
        const renderer = song.playlistPanelVideoRenderer || song.playlistPanelVideoWrapperRenderer.primaryRenderer.playlistPanelVideoRenderer;
        navigate({ watchEndpoint: renderer.navigationEndpoint.watchEndpoint });
        break;
      }
      case "navigate":
        navigate(value);
        break;
      case "togglePlayerPage":
        playerBar().querySelector(".toggle-player-page-button")?.click();
        break;
      case "addToPlaylist":
        openAddToPlaylist(playerBar(), api.getPlayerResponse().videoDetails.videoId);
        break;
    }
  }

  window.__YTMR__ = {
    exec,
    getPlaylists,
    ratioVolume,
    setCustomCss,
    setModernUi,
    setAdBlock: (on) => (adblock.enabled = on),
    navigateHome: () => navigate({ browseEndpoint: { browseId: "FEmusic_home" } }),
    exitFullscreen: () => document.fullscreenElement && document.exitFullscreen(),
    focusSearch: () => document.querySelector("ytmusic-search-box input")?.focus(),
    setAudio: (config) => audio.set(config),
    // Replaced once the window controls exist.
    setMaximized: () => {},
    setFullscreen: (on) => (appFullscreen = on),
    setVolumeSliderPersist,
    togglePlayerPage: () => exec("togglePlayerPage"),
    addToPlaylist: () => exec("addToPlaylist")
  };

  // Apply the theme before YTM paints, and reveal the view as soon as its layout exists
  // instead of waiting for the player.
  // The bridge runs at document creation, before Tauri's IPC and <html> may exist.
  const ipcReady = waitFor(() => window.__TAURI_INTERNALS__);
  // Requested early so the round trip overlaps page loading.
  const configPromise = ipcReady.then(() => invoke("ytm_get_config"));
  if (window.__YTMR_THEME__) waitFor(() => document.documentElement).then(() => setModernUi(window.__YTMR_THEME__));
  // Casting is unsupported. YTM's cast button is hidden from the first paint rather than
  // once the player is ready, so it never shows while the page loads.
  waitFor(() => document.documentElement).then(() => {
    const css = document.createElement("style");
    css.textContent = "ytmusic-cast-button, ytmusic-cast-dialog-renderer { display: none !important; }";
    (document.head || document.documentElement).appendChild(css);
  });
  waitFor(() => document.body && document.querySelector("ytmusic-nav-bar"))
    .then(() => ipcReady)
    .then(() => {
      let ownsHeader = false;
      try {
        createWindowControls();
        ownsHeader = true;
      } catch (e) {
        console.error("YTMusic Redux: window controls failed", e);
      }
      // Without controls in YTM's header the app keeps its own title bar.
      invoke("ytm_loaded", { ownsHeader });
    });

  (async () => {
    const config = await configPromise;
    await waitFor(() => hook().ytmStore && hook().ytmPlayerBar && hook().ytmPlayerBar.playerApi && playerApi().isReady());

    createStyleSheet();
    // YTM's header is the only header, so it carries the back/forward arrows.
    loadMaterialSymbols().then(createNavigationArrows);
    createAdditionalPlayerBarControls();
    ytmStore().dispatch({ type: "SET_CAST_AVAILABLE", payload: false });
    hookPlayerApiEvents();
    const historyButton = document.querySelector("#history-link .history-button");
    if (historyButton) historyButton.style = "display: inline-block !important;";

    continueWhereYouLeftOff(config);
    setVolumeSliderPersist(config.alwaysShowVolumeSlider);
    if (config.ratioVolume) ratioVolume.enable();
    if (config.customCss !== null) setCustomCss(config.customCss);
    setModernUi(config.modernCss);
    audio.set(config.audio);
    autoCloseToasts();
    // As upstream does: after the player is up, so startup is not slowed down.
    setTimeout(() => window.__YTMR_PERF__?.(), 250);
  })();
})();
