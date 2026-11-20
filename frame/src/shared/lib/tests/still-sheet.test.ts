import { describe, expect, it } from "vitest";
import { holdStill, stillHeld } from "../still-sheet";

describe("still sheet", () => {
  it("holds still while any reason holds, and lets go once none does", () => {
    expect(stillHeld()).toBe(false);
    holdStill("motion", true);
    holdStill("game", true);
    expect(stillHeld()).toBe(true);
    holdStill("motion", false);
    // The game still holds it.
    expect(stillHeld()).toBe(true);
    holdStill("game", false);
    expect(stillHeld()).toBe(false);
  });

  it("is attached to the document only while held, where the document can adopt sheets", () => {
    if (typeof document === "undefined" || !("adoptedStyleSheets" in document)) return;
    const before = document.adoptedStyleSheets.length;
    holdStill("game", true);
    expect(document.adoptedStyleSheets.length).toBe(before + 1);
    const rules = [...document.adoptedStyleSheets.at(-1)!.cssRules].map((rule) => rule.cssText);
    expect(rules.join(" ")).toContain("animation-duration");
    // Said twice, attached once.
    holdStill("game", true);
    expect(document.adoptedStyleSheets.length).toBe(before + 1);
    holdStill("game", false);
    expect(document.adoptedStyleSheets.length).toBe(before);
  });
});
