import { describe, expect, it } from "vitest";
import { SIDE_APPS, openTabOf, sideAppPlan } from "../lib/side-apps";

const discord = SIDE_APPS.find((app) => app.id === "discord")!;
const tab = (id: string, url: string | null) => ({ id, url });

describe("sidebar apps", () => {
  it("opens every app on an encrypted address of one of its own hosts", () => {
    for (const app of SIDE_APPS) {
      const address = new URL(app.url);
      expect(address.protocol).toBe("https:");
      expect(app.hosts).toContain(address.hostname);
    }
    expect(new Set(SIDE_APPS.map((app) => app.id)).size).toBe(SIDE_APPS.length);
  });

  it("finds an app's open tab by host, never by a lookalike address", () => {
    const tabs = [
      tab("a", "https://example.com/discord.com"),
      tab("b", "https://discord.com.evil.test/app"),
      tab("c", null),
      tab("d", "not an address"),
      tab("e", "https://discord.com/channels/@me"),
    ];
    expect(openTabOf(discord, tabs)?.id).toBe("e");
    expect(openTabOf(discord, tabs.slice(0, 4))).toBeNull();
  });

  it("opens a new tab beside the page in front, or brings the open one beside it", () => {
    expect(sideAppPlan(discord, [tab("page", "https://example.com/")], "page")).toEqual({
      open: discord.url,
      splitWith: "page",
    });
    const open = [tab("page", "https://example.com/"), tab("app", "https://discord.com/app")];
    expect(sideAppPlan(discord, open, "page")).toEqual({ open: null, splitWith: "app" });
    // Already in front: nothing to do, and never a split with itself.
    expect(sideAppPlan(discord, open, "app")).toEqual({ open: null, splitWith: null });
    // No page in front: it opens on its own.
    expect(sideAppPlan(discord, [], null)).toEqual({ open: discord.url, splitWith: null });
  });
});
