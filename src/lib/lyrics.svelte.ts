import { api, type Lyrics } from "./api";

// Lyrics of the current track, fetched once per video and shared by the now playing view.
export class LyricsTrack {
  data = $state<Lyrics | null>(null);
  loading = $state(false);
  error = $state<string | null>(null);
  private videoId = "";

  async load(videoId: string, retry = false) {
    if (videoId === this.videoId && !retry) return;
    this.videoId = videoId;
    this.data = null;
    this.error = null;
    this.loading = true;
    try {
      const result = await api.getLyrics();
      // The player may have moved on while the request was in flight.
      if (this.videoId === videoId) this.data = result?.videoId === videoId ? result : null;
    } catch (e) {
      if (this.videoId === videoId) this.error = String(e);
    } finally {
      if (this.videoId === videoId) this.loading = false;
    }
  }

  // Index of the synced line playing at `seconds`, or -1 before the first line.
  lineAt(seconds: number): number {
    const lines = this.data?.synced;
    if (!lines) return -1;
    let lo = 0;
    let hi = lines.length - 1;
    let found = -1;
    while (lo <= hi) {
      const mid = (lo + hi) >> 1;
      if (lines[mid].time <= seconds) {
        found = mid;
        lo = mid + 1;
      } else {
        hi = mid - 1;
      }
    }
    return found;
  }
}
