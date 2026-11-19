import { afterEach, describe, expect, it } from "vitest";
import { setTranslations } from "$shared/lib/translations";
import { stepParts, stepText } from "../step-label";

// French is loaded the way the app loads it: its dictionary, in English order.
const french = Object.values(
  import.meta.glob<readonly (string | null)[]>("../../../../messages/fr.json", {
    import: "default",
    query: "?dictionary",
    eager: true,
  }),
)[0]!;
const use = (locale: "en" | "fr") => setTranslations(locale === "fr" ? french : null);
afterEach(() => setTranslations(null));

describe("a step's label", () => {
  it("is read in French, with the page's own names kept word for word", () => {
    use("fr");
    expect(stepText('Click "Press me"')).toBe("Cliquer sur « Press me »");
    expect(stepText("Type “Paris” into “City”")).toBe("Écrire « Paris » dans « City »");
    expect(stepText('Choose "Green" in "Colour"')).toBe("Choisir « Green » dans « Colour »");
    expect(stepText("Go to toureiffel.paris")).toBe("Aller sur toureiffel.paris");
    expect(stepText("Scroll down")).toBe("Descendre dans la page");
    expect(stepText('Press Return in "Search"')).toBe("Valider « Search »");
  });

  it("sets the names off without their quotes, as the row shows them", () => {
    use("fr");
    expect(stepParts('Click "Press me"')).toEqual([
      { text: "Cliquer sur ", named: false },
      { text: "Press me", named: true },
    ]);
    expect(stepParts("Type “Paris” into “City”")).toEqual([
      { text: "Écrire ", named: false },
      { text: "Paris", named: true },
      { text: " dans ", named: false },
      { text: "City", named: true },
    ]);
  });

  it("names a site as an address, without quotes", () => {
    use("fr");
    expect(stepParts("Go to toureiffel.paris")).toEqual([
      { text: "Aller sur toureiffel.paris", named: false },
    ]);
  });

  it("is shown as it came when it is of no known form", () => {
    use("fr");
    const odd = '"Lyon" → Type into "Message"\nsource.example → target.example';
    expect(stepText(odd)).toBe(odd);
    expect(stepParts('Frobnicate "x"')).toEqual([
      { text: "Frobnicate ", named: false },
      { text: "x", named: true },
    ]);
  });

  it("keeps English as native wrote it", () => {
    use("en");
    expect(stepText('Click "Send"')).toBe("Click “Send”");
    expect(stepParts('Click "Send"')).toEqual([
      { text: "Click ", named: false },
      { text: "Send", named: true },
    ]);
  });
});
