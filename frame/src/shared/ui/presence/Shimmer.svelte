<script lang="ts">
  import { watchStill } from "$shared/lib/still";

  /**
   * Words at work: a band of light passes along them. The words are drawn
   * once, in place, and only the light moves: a gradient clipped to the
   * letters slides behind them. Moving a copy of the words instead turned
   * them into a moving bitmap, which Chromium draws without subpixel
   * smoothing and at fractional offsets, so the text looked blurred.
   */
  let { text, running = true }: { text: string; running?: boolean } = $props();

  let host = $state<HTMLElement>();
  let still = $state(true);
  $effect(() => {
    const element = host;
    if (!element || !running) return;
    return watchStill(element, (next) => (still = next));
  });
</script>

<span class="shimmer" class:running data-still={running && still ? "" : undefined} bind:this={host}
  ><span class="words">{text}</span></span
>

<style>
  .shimmer {
    position: relative;
    display: inline-block;
    max-inline-size: 100%;
    overflow: hidden;
    vertical-align: bottom;
    white-space: nowrap;
  }

  .words {
    display: block;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .running .words {
    background-image: linear-gradient(
      90deg,
      var(--shimmer-rest, var(--color-muted)) 0%,
      var(--shimmer-rest, var(--color-muted)) 40%,
      var(--shimmer-lit, var(--color-text)) 50%,
      var(--shimmer-rest, var(--color-muted)) 60%,
      var(--shimmer-rest, var(--color-muted)) 100%
    );
    background-clip: text;
    background-size: 250% 100%;
    color: transparent;
    animation: light 2.4s linear infinite;
  }

  [data-still] .words {
    animation-play-state: paused;
  }

  @media (prefers-reduced-motion: reduce) {
    .running .words {
      animation: none;
      background-position: 100% 0;
    }
  }

  @keyframes light {
    0% {
      background-position: 100% 0;
    }

    72%,
    100% {
      background-position: -60% 0;
    }
  }
</style>
