<script lang="ts">
  import { frameFor, type Quiet, type SkyFrame } from "./dither";
  import type { Box } from "./place";
  import { loadPicture } from "./picture";
  import { frostFrom, printed } from "./print.svelte";
  import type { Scene } from "./scenes";

  let {
    scene,
    width,
    height,
    sky,
    horizon,
    pictureHorizon,
    zoom = 1,
    quiet = null,
    onready,
  }: {
    scene: Scene;
    /** The page's size, in page pixels. */
    width: number;
    height: number;
    /** Where the picture lies on the page (placeSky). */
    sky: Box;
    /** The page's horizon, in page pixels. */
    horizon: number;
    pictureHorizon: number;
    /** How much larger than its own size the page is ever drawn: a page a
     *  camera closes in on is printed for its closest view. */
    zoom?: number;
    /** Where the page's type stands, so the sky is held quieter behind it. */
    quiet?: Quiet | null;
    /** Called once the scene first shows. */
    onready?: (scene: Scene) => void;
  } = $props();

  let density = $state(globalThis.devicePixelRatio || 1);
  // Moving the window to a screen of another density reprints it there.
  $effect(() => {
    const query = matchMedia(`(resolution: ${density}dppx)`);
    const moved = () => (density = globalThis.devicePixelRatio || 1);
    query.addEventListener("change", moved, { once: true });
    return () => query.removeEventListener("change", moved);
  });

  let canvas = $state<HTMLCanvasElement>();
  let scale = $derived(density * zoom);
  let size = $derived(frameFor(width, height, scale));
  let frame = $derived<SkyFrame>({ ...size, density: scale, sky, horizon, pictureHorizon, quiet });

  // How long a size must hold before it is printed again.
  const SETTLE_MS = 120;
  // The scene the canvas already shows, once it shows one.
  let shownScene: Scene | null = null;

  // The first print is made as soon as its picture is read, with no animation
  // frame waited for, so a window still hidden is printed before it is shown.
  // After that a window being resized is laid out at every size on the way,
  // and printing each one (hundreds of thousands of dots) made the drag
  // stutter: the print already there is stretched meanwhile, and the new size
  // is printed once it holds still. Nothing runs at rest.
  $effect(() => {
    const wanted = frame;
    const target = canvas;
    const wantedScene = scene;
    if (!target || wanted.columns <= 0 || wanted.rows <= 0 || wanted.sky.w <= 0) return;
    let live = true;
    const print = () =>
      void loadPicture(wantedScene).then(
        (picture) => {
          if (!live) return;
          const pixels = printed(wantedScene, picture, wanted);
          if (target.width !== wanted.columns) target.width = wanted.columns;
          if (target.height !== wanted.rows) target.height = wanted.rows;
          target.getContext("2d")?.putImageData(pixels, 0, 0);
          shownScene = wantedScene;
          frostFrom(wantedScene, target);
          onready?.(wantedScene);
        },
        () => {
          if (live) onready?.(wantedScene);
        },
      );
    // Another scene is another picture: only the same one stretches.
    const timer = shownScene === wantedScene ? setTimeout(print, SETTLE_MS) : undefined;
    if (timer === undefined) print();
    return () => {
      live = false;
      clearTimeout(timer);
    };
  });
</script>

<!-- One dot of the print is a whole number of device pixels, so the screen
     stays crisp at any size: the canvas is the dots, scaled up without
     smoothing. -->
<canvas
  bind:this={canvas}
  class="sky-print"
  aria-hidden="true"
  style:width={`${(size.columns * size.dot) / scale}px`}
  style:height={`${(size.rows * size.dot) / scale}px`}
></canvas>

<style>
  .sky-print {
    position: absolute;
    inset-block-start: 0;
    inset-inline-start: 0;
    image-rendering: pixelated;
    pointer-events: none;
  }
</style>
