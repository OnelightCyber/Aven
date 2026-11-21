import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import {
  MAX_NAME_CHARS,
  SETUP_KEYS,
  admit,
  changes,
  cleanName,
  inUse,
  newId,
  parse,
  serialize,
  snapshot,
  uniqueName,
  type Setup,
} from "../model";

const night: Setup = {
  id: "00000000000000aa",
  name: "Night",
  settings: { appearance: "dark", "ui.font": "mono" },
};

describe("setup model", () => {
  it("carries exactly the settings native admits in a setup", () => {
    const native = readFileSync("../crates/aven-core/src/setups.rs", "utf8");
    const list = /pub const KEYS: &\[&str\] = &\[([^\]]*)\];/u.exec(native)?.[1] ?? "";
    const keys = [...list.matchAll(/"([^"]+)"/gu)].map((match) => match[1]);
    expect(keys.length).toBeGreaterThan(0);
    expect([...SETUP_KEYS]).toEqual(keys);
  });

  it("reads the stored list whole, and nothing from one it cannot read", () => {
    expect(parse("")).toEqual([]);
    expect(parse(serialize([night]))).toEqual([night]);
    expect(serialize([])).toBe("");
    for (const broken of [
      "not json",
      "{}",
      '[{"id":"short","name":"Night","settings":{}}]',
      '[{"id":"00000000000000aa","name":"","settings":{}}]',
      '[{"id":"00000000000000aa","name":"Night","settings":{}},7]',
    ]) {
      expect(parse(broken)).toEqual([]);
    }
  });

  it("keeps only what a setup carries, as text", () => {
    expect(
      admit({
        appearance: "dark",
        "privacy.smartscreen": "false",
        "ui.newtab-clock": true,
        "ui.language": "fr",
      }),
    ).toEqual({ appearance: "dark" });
    expect(admit(null)).toEqual({});
    expect(admit("appearance")).toEqual({});
  });

  it("cleans a name to what is shown and stored", () => {
    expect(cleanName("  Night\u0007 one \n")).toBe("Night one");
    expect([...cleanName("n".repeat(MAX_NAME_CHARS * 2))]).toHaveLength(MAX_NAME_CHARS);
    expect(cleanName(" \t ")).toBe("");
  });

  it("sets a repeated name apart with a number", () => {
    expect(uniqueName("Night", [])).toBe("Night");
    expect(uniqueName("Night", ["Night"])).toBe("Night 2");
    expect(uniqueName("Night", ["Night", "Night 2"])).toBe("Night 3");
    const long = "n".repeat(MAX_NAME_CHARS);
    const numbered = uniqueName(long, [long]);
    expect([...numbered]).toHaveLength(MAX_NAME_CHARS);
    expect(numbered.endsWith(" 2")).toBe(true);
  });

  it("makes ids of sixteen lowercase hexadecimal characters", () => {
    expect(newId(new Uint8Array([0, 1, 2, 255, 16, 171, 9, 128]))).toBe("000102ff10ab0980");
    expect(newId()).toMatch(/^[0-9a-f]{16}$/u);
    expect(newId()).not.toBe(newId());
  });

  it("snapshots every setting a setup carries", () => {
    const taken = snapshot((key) => `value of ${key}`);
    expect(Object.keys(taken)).toEqual([...SETUP_KEYS]);
    expect(taken.appearance).toBe("value of appearance");
  });

  it("lists what applying would change, and is in use once nothing would", () => {
    const now: Record<string, string> = { appearance: "light", "ui.font": "mono" };
    const read = (key: string) => now[key] ?? "default";
    expect(changes(night, read)).toEqual([["appearance", "dark"]]);
    expect(inUse(night, read)).toBe(false);
    now.appearance = "dark";
    expect(changes(night, read)).toEqual([]);
    expect(inUse(night, read)).toBe(true);
    expect(inUse({ ...night, settings: {} }, read)).toBe(false);
  });
});
