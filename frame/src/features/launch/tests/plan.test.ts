import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { NAME_BOX, NAME_DOTS } from "../lib/dots";
import { LAUNCH, launchWanted, nameWidth, plan } from "../lib/plan";

describe("launch formation", () => {
  it("draws exactly the dots of the published wordmark", () => {
    const svg = readFileSync("public/aven-wordmark.svg", "utf8");
    const [, width, height] = /viewBox="0 0 (\d+) (\d+)"/u.exec(svg) ?? [];
    expect([Number(width), Number(height)]).toEqual([NAME_BOX.width, NAME_BOX.height]);
    const circles = [...svg.matchAll(/cx="([\d.]+)" cy="([\d.]+)" r="([\d.]+)"/gu)].map((match) =>
      match.slice(1).map(Number),
    );
    expect(circles).toEqual(NAME_DOTS.map((dot) => [...dot]));
  });

  it("brings every dot up close to its place, so the name is what is seen", () => {
    for (const dot of plan(468).dots) {
      expect(Math.hypot(dot.dx, dot.dy)).toBeLessThanOrEqual(18);
    }
  });

  it("lands every dot inside the name, the same way on every launch", () => {
    const width = 468;
    const first = plan(width);
    expect(plan(width)).toEqual(first);
    expect(first.dots).toHaveLength(NAME_DOTS.length);
    for (const dot of first.dots) {
      expect(dot.left).toBeGreaterThanOrEqual(-dot.size);
      expect(dot.left + dot.size).toBeLessThanOrEqual(width + dot.size);
      expect(dot.top + dot.size).toBeLessThanOrEqual(first.height + dot.size);
      expect(dot.size).toBeGreaterThan(0);
    }
  });

  it("finishes every movement before the cover fades, and the cover before it is done", () => {
    const { dots } = plan(468);
    const landed = Math.max(...dots.map((dot) => dot.delay + LAUNCH.land));
    expect(landed).toBeLessThanOrEqual(LAUNCH.fadeAt);
    const glinted = Math.max(...dots.map((dot) => dot.glint + LAUNCH.glint));
    expect(glinted).toBeLessThanOrEqual(LAUNCH.doneAt);
    expect(LAUNCH.fadeAt + LAUNCH.fade).toBeLessThanOrEqual(LAUNCH.doneAt);
    // The name stays whole a while before it fades, and the page waits at
    // most a second and a half after the window shows.
    expect(LAUNCH.fadeAt - landed).toBeGreaterThanOrEqual(400);
    expect(LAUNCH.doneAt).toBeLessThanOrEqual(1500);
  });

  it("draws the name at most 468 pixels wide, less in a narrow window", () => {
    expect(nameWidth(1920)).toBe(468);
    expect(nameWidth(800)).toBe(336);
    expect(nameWidth(0)).toBe(0);
    expect(nameWidth(-10)).toBe(0);
  });

  it("plays only when it is wanted, never with reduced motion or a game in front", () => {
    expect(launchWanted({ enabled: true, reducedMotion: false, game: false })).toBe(true);
    expect(launchWanted({ enabled: false, reducedMotion: false, game: false })).toBe(false);
    expect(launchWanted({ enabled: true, reducedMotion: true, game: false })).toBe(false);
    expect(launchWanted({ enabled: true, reducedMotion: false, game: true })).toBe(false);
  });
});
