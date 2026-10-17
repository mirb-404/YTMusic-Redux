// Interpolates playback position between player-state events so progress UIs move smoothly.
export class ProgressClock {
  now = $state(performance.now());
  private anchor = $state({ progress: 0, at: performance.now() });
  private frame = 0;

  start() {
    const tick = () => {
      this.now = performance.now();
      this.frame = requestAnimationFrame(tick);
    };
    this.frame = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(this.frame);
  }

  sync(progress: number) {
    this.anchor = { progress, at: performance.now() };
  }

  elapsed(playing: boolean, duration: number) {
    const value = this.anchor.progress + (playing ? (this.now - this.anchor.at) / 1000 : 0);
    return duration > 0 ? Math.min(duration, value) : value;
  }
}
