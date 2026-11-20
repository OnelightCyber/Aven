import { WORDMARK } from "./wordmark";

/** The pitch between the name's dots, in CSS pixels, and how far a whole
 *  cell's dot reaches: a full stroke closes up but still reads as dots. The
 *  onboarding's formation lands on this same screen. */
export const NAME_PITCH = 3.4;
export const NAME_REACH = 0.58;

/** A dot of the name: its centre and radius, in the box's own pixels. */
export type NameDot = readonly [x: number, y: number, r: number];

/**
 * The name as a halftone, the way it forms out of the sky: a dot at the
 * centre of every cell its letters touch, as large as the share of the cell
 * they cover. Measured over four samples a pixel so each dot grades
 * smoothly. `pitch` is in the box's pixels.
 */
const NAME_DOTS = new Map<string, ReturnType<typeof nameDotsFresh>>();

/** The name's dots for a box, kept per size: rasterizing it reads back
 *  megabytes of pixels, and a page asks again on every relayout. */
export function nameDots(...box: Parameters<typeof nameDotsFresh>) {
  const key = box.join("x");
  let dots = NAME_DOTS.get(key);
  if (!dots) {
    // A window dragged through many sizes must not keep them all.
    if (NAME_DOTS.size >= 8) NAME_DOTS.clear();
    dots = nameDotsFresh(...box);
    NAME_DOTS.set(key, dots);
  }
  return dots;
}

/** Two decimals: a hundredth of a pixel is below what any screen shows. */
const short = (value: number) => String(Math.round(value * 100) / 100);

/**
 * The dots as one path, each a circle drawn as two arcs in the same turning
 * sense, so dots that touch stay filled. One element where there were
 * hundreds: the page styles, lays out and paints one shape, and a `<use>` of
 * it copies one.
 */
export function dotsPath(dots: readonly NameDot[]): string {
  let path = "";
  for (const [x, y, r] of dots) {
    if (!(r > 0)) continue;
    path += `M${short(x - r)} ${short(y)}a${short(r)} ${short(r)} 0 1 0 ${short(2 * r)} 0a${short(r)} ${short(r)} 0 1 0 ${short(-2 * r)} 0Z`;
  }
  return path;
}

function nameDotsFresh(width: number, height: number, pitch = NAME_PITCH): NameDot[] {
  if (typeof document === "undefined" || width <= 0 || height <= 0 || pitch <= 0) return [];
  const fine = 4;
  const off = document.createElement("canvas");
  off.width = Math.ceil(width * fine);
  off.height = Math.ceil(height * fine);
  const context = off.getContext("2d", { willReadFrequently: true });
  if (!context) return [];
  context.scale(fine, fine);
  context.scale(width / WORDMARK.width, height / WORDMARK.height);
  context.translate(-WORDMARK.x, -WORDMARK.y);
  context.scale(WORDMARK.scaleX, WORDMARK.scaleY);
  context.fill(new Path2D(WORDMARK.d));
  const data = context.getImageData(0, 0, off.width, off.height).data;
  const dots: NameDot[] = [];
  const cell = pitch * fine;
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
        (left + cell / 2) / fine,
        (top + cell / 2) / fine,
        pitch * NAME_REACH * Math.sqrt(share),
      ]);
    }
  // The buffer goes back now rather than when it is collected.
  off.width = 0;
  off.height = 0;
  return dots;
}
