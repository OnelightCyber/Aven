import { describe, expect, it } from "vitest";
import type { WorkModelEntry } from "$shared/ipc/bindings";
import { tierByName, tiers } from "../components/bar/model-tier";

const entry = (id: string, output: number | null): WorkModelEntry =>
  ({
    id,
    display_name: id,
    model: { provider: "compatible", model: id },
    price: output === null ? null : { input: 0, output },
  }) as unknown as WorkModelEntry;

describe("a model's class", () => {
  it("comes from its name when it costs nothing or has no price", () => {
    const found = tiers([
      {
        entries: [
          entry("claude-code/haiku", 0),
          entry("claude-code/sonnet", 0),
          entry("claude-code/opus", null),
          entry("my-own-model", 0),
        ],
      },
    ]);
    expect(found).toEqual({
      "claude-code/haiku": "light",
      "claude-code/sonnet": "balanced",
      "claude-code/opus": "flagship",
    });
  });

  it("still comes from what it costs when it has a price", () => {
    const found = tiers([{ entries: [entry("cheap-opus", 2_000_000), entry("dear", 30_000_000)] }]);
    expect(found["cheap-opus"]).toBe("light");
    expect(found["dear"]).toBe("flagship");
  });

  it("is not guessed from a word inside another one", () => {
    expect(tierByName(entry("minimal-reasoner", null))).toBeNull();
    expect(tierByName(entry("gpt-5-mini", null))).toBe("light");
  });
});
