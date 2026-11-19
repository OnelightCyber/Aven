import { NAME_PITCH, NAME_REACH } from "$shared/lib/halftone";

export type Box = { x: number; y: number; w: number; h: number };

type Dot = {
  sx: number;
  sy: number;
  tx: number;
  ty: number;
  r: number;
  delay: number;
  span: number;
  sway: number;
  phase: number;
};

/** The name's screen, in stage pixels: the same one the new tab draws. */
const PITCH = NAME_PITCH;
const REACH = NAME_REACH;
/** The last dot lands by then: each waits up to a quarter of it and
 *  travels for about half. */
const GATHER = 2500;
/** The name stands as a screen for a moment before it counts as formed. */
const HOLD = 220;

/**
 * The name drawn out of the sky's own halftone: dots rise from the screen
 * below the horizon, drift up on the air and settle on a screen of the
 * letters, and stay there: the name is made of the sky's dots, as on the
 * new tab. It runs once, a few seconds; then the loop ends and the last
 * frame stays drawn until the returned stop clears it.
 */
export function formName(
  canvas: HTMLCanvasElement,
  options: {
    stage: { width: number; height: number };
    /** Where the name stands, in stage pixels. */
    box: Box;
    /** The letters, as one path in the coordinates `place` maps from. */
    path: Path2D;
    place: (context: CanvasRenderingContext2D) => void;
    /** Device pixels per stage pixel. */
    density: number;
    color: string;
    onformed: () => void;
  },
): () => void {
  const { stage, box, density } = options;
  canvas.width = Math.round(stage.width * density);
  canvas.height = Math.round(stage.height * density);
  const context = canvas.getContext("2d");
  if (!context) {
    options.onformed();
    return () => {};
  }
  context.setTransform(density, 0, 0, density, 0, 0);

  // Each dot starts in the band under the name, from roughly below where it
  // lands, so the screen seems to give the letters up rather than fling
  // dots across the sky.
  const floor = Math.min(stage.height, box.y + box.h + 60);
  const dots: Dot[] = screen(options).map(([tx, ty, r]) => {
    const along = (tx - box.x) / box.w;
    return {
      sx: tx + (Math.random() - 0.5) * box.w * 0.5,
      sy: floor + Math.random() * (stage.height - floor + 40),
      tx,
      ty,
      r,
      // A wave from the middle of the name outwards, loosened by chance.
      delay: GATHER * (Math.abs(along - 0.5) * 0.2 + Math.random() * 0.16),
      span: GATHER * (0.5 + Math.random() * 0.14),
      sway: 14 + Math.random() * 40,
      phase: Math.random() * Math.PI * 2,
    };
  });
  const landed = dots.reduce((end, dot) => Math.max(end, dot.delay + dot.span), 0);

  const [r, g, b] = rgb(options.color);
  const shades = [0.25, 0.5, 0.75, 1];
  const paths = shades.map(() => new Path2D());
  let started = 0;
  let frame = 0;
  let stopped = false;

  function step(now: number) {
    if (stopped) return;
    if (!started) started = now;
    const age = now - started;
    for (let index = 0; index < paths.length; index++) paths[index] = new Path2D();

    for (const dot of dots) {
      const t = Math.min(1, Math.max(0, (age - dot.delay) / dot.span));
      if (t <= 0) continue;
      const e = t * t * t * (t * (t * 6 - 15) + 10);
      const x = dot.sx + (dot.tx - dot.sx) * e + Math.sin(dot.phase + t * 4.2) * dot.sway * (1 - e);
      const y = dot.sy + (dot.ty - dot.sy) * e;
      const radius = 0.55 + (dot.r - 0.55) * e * e;
      // Faint while it travels, whole once it lands.
      const shade = Math.min(1, t * 3) * (0.45 + 0.55 * e);
      const bucket = Math.min(shades.length - 1, Math.floor(shade * shades.length));
      if (shade <= 0.02 || radius <= 0.1) continue;
      paths[bucket]!.moveTo(x + radius, y);
      paths[bucket]!.arc(x, y, radius, 0, Math.PI * 2);
    }

    context!.clearRect(0, 0, stage.width, stage.height);
    for (let index = 0; index < paths.length; index++) {
      context!.fillStyle = `rgba(${r},${g},${b},${shades[index]!.toFixed(2)})`;
      context!.fill(paths[index]!);
    }

    if (age >= landed + HOLD) {
      // Every dot has landed: the last frame stays as the name, and nothing
      // keeps running.
      options.onformed();
      return;
    }
    frame = requestAnimationFrame(step);
  }

  function stop() {
    stopped = true;
    cancelAnimationFrame(frame);
    // An empty canvas gives its window-sized buffer back now; a removed one
    // keeps it until it is collected.
    canvas.width = 0;
    canvas.height = 0;
  }

  frame = requestAnimationFrame(step);
  return stop;
}

/** The letters as a halftone: a dot at the centre of every cell they touch,
 *  as large as the share of the cell they cover, in stage pixels. */
function screen(options: {
  box: Box;
  path: Path2D;
  place: (context: CanvasRenderingContext2D) => void;
}): [number, number, number][] {
  const { box } = options;
  // Drawn at four times the stage's pixels, so a cell's cover is measured
  // over enough samples to grade its dot smoothly.
  const fine = 4;
  const off = document.createElement("canvas");
  off.width = Math.ceil(box.w * fine);
  off.height = Math.ceil(box.h * fine);
  const context = off.getContext("2d", { willReadFrequently: true });
  if (!context) return [];
  context.scale(fine, fine);
  options.place(context);
  context.fill(options.path);
  const data = context.getImageData(0, 0, off.width, off.height).data;
  const dots: [number, number, number][] = [];
  const cell = PITCH * fine;
  for (let top = 0; top < off.height; top += cell)
    for (let left = 0; left < off.width; left += cell) {
      let covered = 0;
      let count = 0;
      for (let y = Math.floor(top); y < Math.min(off.height, top + cell); y += 2)
        for (let x = Math.floor(left); x < Math.min(off.width, left + cell); x += 2) {
          covered += data[(y * off.width + x) * 4 + 3]!;
          count++;
        }
      const share = covered / (count * 255);
      if (share < 0.08) continue;
      dots.push([
        box.x + (left + cell / 2) / fine,
        box.y + (top + cell / 2) / fine,
        PITCH * REACH * Math.sqrt(share),
      ]);
    }
  return dots;
}

/** A CSS colour as RGB, by letting the canvas normalise it. */
function rgb(color: string): [number, number, number] {
  const probe = document.createElement("canvas").getContext("2d");
  if (!probe) return [255, 255, 255];
  probe.fillStyle = color;
  const value = probe.fillStyle;
  if (value.startsWith("#") && value.length === 7)
    return [1, 3, 5].map((at) => parseInt(value.slice(at, at + 2), 16)) as [number, number, number];
  const parts = value.match(/\d+(\.\d+)?/gu)?.map(Number) ?? [255, 255, 255];
  return [parts[0] ?? 255, parts[1] ?? 255, parts[2] ?? 255];
}
