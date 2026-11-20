import { describe, expect, it } from "vitest";
import { dotsPath } from "../halftone";

describe("dotsPath", () => {
  it("draws each dot as one closed circle of two arcs turning the same way", () => {
    expect(dotsPath([[10, 20, 2]])).toBe("M8 20a2 2 0 1 0 4 0a2 2 0 1 0 -4 0Z");
  });

  it("joins every dot into one path, rounded to hundredths, and skips empty ones", () => {
    const path = dotsPath([
      [1.234567, 2, 0.5],
      [5, 5, 0],
      [7, 8, 1],
    ]);
    expect(path).toBe(
      "M0.73 2a0.5 0.5 0 1 0 1 0a0.5 0.5 0 1 0 -1 0ZM6 8a1 1 0 1 0 2 0a1 1 0 1 0 -2 0Z",
    );
    expect(dotsPath([])).toBe("");
  });
});
