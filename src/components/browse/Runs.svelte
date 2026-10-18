<script lang="ts">
  import { router, type Run } from "../../lib/music.svelte";

  // Text runs whose artist, album and playlist parts are links.
  let { runs }: { runs: Run[] } = $props();
</script>

{#each runs as r, i (i)}{#if r.endpoint?.browseEndpoint}<button
      class="link"
      onclick={(e) => {
        e.stopPropagation();
        router.open(r.endpoint);
      }}>{r.text}</button
    >{:else}{r.text}{/if}{/each}

<style>
  .link { display: inline; padding: 0; font: inherit; color: inherit; cursor: pointer; text-align: left; }
  .link:hover { color: var(--text); text-decoration: underline; text-underline-offset: 2px; }
</style>
