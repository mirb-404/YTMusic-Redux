import { invoke } from "@tauri-apps/api/core";
import type { Thumbnail } from "./api";

// Models of src-tauri/src/music/parse.rs. Endpoints are YTM's own JSON, trimmed.

export type Endpoint = {
  browseEndpoint?: { browseId: string; params?: string; pageType?: string };
  watchEndpoint?: { videoId?: string; playlistId?: string; params?: string; musicVideoType?: string };
  watchPlaylistEndpoint?: { playlistId: string; params?: string };
  searchEndpoint?: { query: string; params?: string };
};
export type Run = { text: string; endpoint: Endpoint | null };
export type ItemKind = "song" | "video" | "album" | "playlist" | "artist" | "episode" | "podcast" | "category" | "other";
export type Item = {
  kind: ItemKind;
  title: string;
  subtitle: Run[];
  thumbnails: Thumbnail[];
  endpoint: Endpoint | null;
  play: Endpoint | null;
  videoId: string | null;
  duration: string | null;
  index: string | null;
  aspect: "square" | "wide" | "circle";
  color: string | null;
};
export type Continuation = { token: string; query: boolean };
export type Section = {
  kind: "carousel" | "list" | "grid" | "card" | "description";
  title: string;
  strapline: string | null;
  more: Endpoint | null;
  items: Item[];
  text: string | null;
  rows: boolean;
  continuation: Continuation | null;
};
export type Header = {
  kind: "plain" | "collection" | "artist";
  title: string;
  subtitle: Run[];
  secondSubtitle: Run[];
  description: string | null;
  thumbnails: Thumbnail[];
  play: Endpoint | null;
  shuffle: Endpoint | null;
  radio: Endpoint | null;
};
export type Chip = { text: string; endpoint: Endpoint | null; selected: boolean };
export type Page = { header: Header | null; chips: Chip[]; sections: Section[]; continuation: Continuation | null };
export type Suggestions = { queries: Run[][]; items: Item[] };
export type GuideEntry = { title: string; subtitle: string; endpoint: Endpoint; play: Endpoint | null };

export const music = {
  browse: (endpoint: Endpoint) => invoke<Page>("music_browse", { endpoint }),
  search: (query: string, params?: string) => invoke<Page>("music_search", { query, params: params ?? null }),
  continue: (continuation: Continuation) => invoke<Page>("music_continue", { continuation }),
  suggestions: (input: string) => invoke<Suggestions>("music_suggestions", { input }),
  guide: () => invoke<GuideEntry[]>("music_guide"),
  signedIn: () => invoke<boolean>("music_signed_in"),
  play: (endpoint: Endpoint) => invoke<void>("music_play", { endpoint }),
  signIn: () => invoke<void>("music_sign_in")
};

/** The playing track, kept by the shell, so rows and cards can mark it. */
export const nowPlaying = $state({ id: "", playing: false });

export const browseTo = (browseId: string, params?: string): Endpoint => ({ browseEndpoint: { browseId, ...(params ? { params } : {}) } });
export const HOME = browseTo("FEmusic_home");
export const EXPLORE = browseTo("FEmusic_explore");
export const LIBRARY = browseTo("FEmusic_library_landing");
export const HISTORY = browseTo("FEmusic_history");

const playable = (e: Endpoint | null | undefined) => !!(e?.watchEndpoint || e?.watchPlaylistEndpoint);

/** Artwork at about `size` CSS pixels, sharp on high-DPI screens. */
export function art(thumbs: Thumbnail[] | undefined, size: number): string | null {
  if (!thumbs?.length) return null;
  const want = Math.round(size * Math.min(2, window.devicePixelRatio || 1));
  const best = [...thumbs].sort((a, b) => b.width - a.width)[0];
  // googleusercontent images are served at the size their URL asks for.
  if (/=w\d+-h\d+/.test(best.url) && best.width > 0 && best.height > 0) {
    const scale = want / Math.max(best.width, best.height);
    const w = Math.round(best.width * scale);
    const h = Math.round(best.height * scale);
    return best.url.replace(/=w\d+-h\d+[^/]*$/, `=w${w}-h${h}-l90-rj`);
  }
  return best.url;
}

// --- navigation

export type Route = { kind: "browse"; endpoint: Endpoint } | { kind: "search"; query: string; params?: string };

type Entry = {
  route: Route;
  page: Page | null;
  error: string | null;
  scroll: number;
  /** Bumped when continuations append, so views refresh. */
  version: number;
  loadingMore: boolean;
};

const routeKey = (r: Route) => JSON.stringify(r.kind === "browse" ? [r.endpoint.browseEndpoint?.browseId, r.endpoint.browseEndpoint?.params] : [r.query, r.params]);

/** Pages already loaded, so going back (or to a page seen moments ago) is instant. */
const cache = new Map<string, { page: Page; at: number }>();
const CACHE_MS = 5 * 60_000;

async function load(route: Route): Promise<Page> {
  const key = routeKey(route);
  const hit = cache.get(key);
  if (hit && Date.now() - hit.at < CACHE_MS) return hit.page;
  const page = route.kind === "browse" ? await music.browse(route.endpoint) : await music.search(route.query, route.params);
  cache.set(key, { page, at: Date.now() });
  return page;
}

class Router {
  entries = $state<Entry[]>([]);
  index = $state(-1);
  current = $derived(this.entries[this.index] ?? null);
  canBack = $derived(this.index > 0);
  canForward = $derived(this.index < this.entries.length - 1);
  /** Read by the scrolling content area, written when leaving a page. */
  scroller: HTMLElement | null = null;

  go(route: Route) {
    if (this.current && routeKey(this.current.route) === routeKey(route)) {
      this.scroller?.scrollTo({ top: 0, behavior: "smooth" });
      return;
    }
    this.saveScroll();
    const entry: Entry = { route, page: null, error: null, scroll: 0, version: 0, loadingMore: false };
    this.entries = [...this.entries.slice(0, this.index + 1), entry];
    this.index = this.entries.length - 1;
    this.fetch(this.entries[this.index]);
  }

  /** Follows an item or run: albums, artists and playlists open, songs and videos play. */
  open(endpoint: Endpoint | null | undefined) {
    if (!endpoint) return;
    if (endpoint.browseEndpoint) this.go({ kind: "browse", endpoint });
    else if (endpoint.searchEndpoint) this.go({ kind: "search", query: endpoint.searchEndpoint.query, params: endpoint.searchEndpoint.params });
    else if (playable(endpoint)) music.play(endpoint);
  }

  /** Loads a page into the cache without showing it. */
  prefetch(route: Route) {
    load(route).catch(() => {});
  }

  back() {
    if (!this.canBack) return;
    this.saveScroll();
    this.index--;
  }

  forward() {
    if (!this.canForward) return;
    this.saveScroll();
    this.index++;
  }

  reload() {
    const entry = this.current;
    if (!entry) return;
    cache.delete(routeKey(entry.route));
    entry.page = null;
    entry.error = null;
    this.fetch(entry);
  }

  private saveScroll() {
    if (this.current && this.scroller) this.current.scroll = this.scroller.scrollTop;
  }

  private async fetch(entry: Entry) {
    try {
      entry.page = await load(entry.route);
    } catch (e) {
      entry.error = String(e);
    }
  }

  /** Appends the next part of the page, or of one of its shelves. */
  async more(section?: Section) {
    const entry = this.current;
    const page = entry?.page;
    const continuation = section ? section.continuation : page?.continuation;
    if (!entry || !page || !continuation || entry.loadingMore) return;
    entry.loadingMore = true;
    try {
      const next = await music.continue(continuation);
      if (section) {
        const added = next.sections[0];
        section.items = [...section.items, ...(added?.items ?? [])];
        section.continuation = added?.continuation ?? null;
      } else {
        page.sections = [...page.sections, ...next.sections];
        page.continuation = next.continuation;
      }
      entry.version++;
    } catch {
      // Leave the continuation so scrolling tries again.
    } finally {
      entry.loadingMore = false;
    }
  }
}

export const router = new Router();
