<script lang="ts">
  import { onDestroy } from "svelte";
  import { holdChromeModal } from "$domain/surface";
  import { LAUNCH, nameWidth, plan } from "../lib/plan";

  /**
   * The name forming out of scattered dots as the window shows. It is drawn
   * before the window is revealed, holding still, and runs once `running`
   * says the window is on screen. Pages are native views above the chrome,
   * so they are held off the screen for its second, then let through. A
   * click or a key ends it at once.
   */
  let { running, onfade, ondone }: { running: boolean; onfade: () => void; ondone: () => void } =
    $props();

  const width = nameWidth(window.innerWidth);
  const { height, dots } = plan(width);
  let fading = $state(false);
  let release: (() => void) | null = holdChromeModal();
  const timers: ReturnType<typeof setTimeout>[] = [];
  let ended = false;

  function fade() {
    if (fading) return;
    fading = true;
    onfade();
  }

  function end() {
    if (ended) return;
    ended = true;
    fade();
    release?.();
    release = null;
    ondone();
  }

  $effect(() => {
    if (!running) return;
    timers.push(setTimeout(fade, LAUNCH.fadeAt), setTimeout(end, LAUNCH.doneAt));
    window.addEventListener("pointerdown", end, true);
    window.addEventListener("keydown", end, true);
    return () => {
      window.removeEventListener("pointerdown", end, true);
      window.removeEventListener("keydown", end, true);
    };
  });

  onDestroy(() => {
    for (const timer of timers) clearTimeout(timer);
    release?.();
    release = null;
  });
</script>

<div
  class="launch"
  class:running
  class:fading
  aria-hidden="true"
  style:--land="{LAUNCH.land}ms"
  style:--glint="{LAUNCH.glint}ms"
  style:--fade="{LAUNCH.fade}ms"
>
  <div class="name" style:width="{width}px" style:height="{height}px">
    {#each dots as dot (dot.key)}
      <span
        class="dot"
        style:left="{dot.left}px"
        style:top="{dot.top}px"
        style:width="{dot.size}px"
        style:height="{dot.size}px"
        style:--dx="{dot.dx}px"
        style:--dy="{dot.dy}px"
        style:--delay="{dot.delay}ms"
        style:--glint-at="{dot.glint}ms"><span class="core"></span></span
      >
    {/each}
  </div>
</div>

<style>
  /* Above everything the window draws; the page itself is held off. */
  .launch {
    position: fixed;
    inset: 0;
    z-index: 2147483000;
    display: grid;
    place-items: center;
    background: var(--color-chrome);
  }

  .name {
    position: relative;
    translate: 0 -4vh;
  }

  /* Translate, scale and opacity only, one animation an element: the
     compositor then moves every dot itself, so the startup work running
     beside it cannot hold a frame. Two animations of the same property on
     one element are refused by the compositor and fall back to the main
     thread, which is why the glint is on the dot's core. Nothing here asks
     the graphics driver for a program it lacks. */
  .dot {
    position: absolute;
    opacity: 0;
  }

  .core {
    position: absolute;
    inset: 0;
    border-radius: 50%;
    background: var(--color-text);
  }

  .running .dot {
    animation: form var(--land) cubic-bezier(0.16, 1, 0.3, 1) var(--delay) both;
  }

  .running .core {
    animation: glint var(--glint) ease-out var(--glint-at) both;
  }

  .fading {
    animation: fade var(--fade) cubic-bezier(0.4, 0, 1, 1) both;
  }

  @keyframes form {
    0% {
      translate: var(--dx) var(--dy);
      opacity: 0;
      scale: 0.3;
    }

    55% {
      opacity: 1;
    }

    100% {
      translate: 0 0;
      opacity: 0.9;
      scale: 1;
    }
  }

  @keyframes glint {
    0%,
    100% {
      scale: 1;
    }

    40% {
      scale: 1.5;
    }
  }

  @keyframes fade {
    to {
      opacity: 0;
      scale: 1.02;
    }
  }
</style>
