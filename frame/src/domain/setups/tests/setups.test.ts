import { beforeEach, expect, test, vi } from "vitest";

// A preferences store in miniature: values, a write that can be refused,
// and the failure flag the real one raises.
const store = vi.hoisted(() => ({
  values: {} as Record<string, string>,
  refuse: new Set<string>(),
  failed: false,
  writes: [] as [string, string][],
}));
vi.mock("$domain/preferences", () => ({
  preferences: {
    value: (key: string) => store.values[key] ?? "",
    saving: () => false,
    saveFailed: () => store.failed,
    set: (key: string, next: string) => {
      store.writes.push([key, next]);
      store.failed = store.refuse.has(key);
      if (!store.failed) store.values[key] = next;
      return Promise.resolve();
    },
  },
}));
const native = vi.hoisted(() => ({
  export: vi.fn(),
  import: vi.fn(),
  list: vi.fn(),
  take: vi.fn(),
}));
vi.mock("$shared/ipc/bindings", async () => {
  const { mockBindings } = await import("$shared/testing/bindings");
  return mockBindings({
    setupsExport: native.export as never,
    setupsImport: native.import as never,
    setupsGalleryList: native.list as never,
    setupsGalleryTake: native.take as never,
  });
});

import { MAX_SETUPS, setups } from "..";

beforeEach(() => {
  store.values = { appearance: "dark", "ui.font": "mono", "ui.setups": "" };
  store.refuse = new Set();
  store.failed = false;
  store.writes = [];
  native.export.mockReset();
  native.import.mockReset();
  native.list.mockReset();
  native.take.mockReset();
});

test("saves how Aven is set up now, and shows it as in use until something changes", async () => {
  expect(await setups.save("  Night  ")).toBe("saved");
  const [night] = setups.list();
  expect(night?.name).toBe("Night");
  expect(night?.settings.appearance).toBe("dark");
  expect(setups.inUse(night!)).toBe(true);
  store.values.appearance = "light";
  expect(setups.inUse(night!)).toBe(false);
  // A second one under the same name is told apart.
  expect(await setups.save("Night")).toBe("saved");
  expect(setups.list().map((setup) => setup.name)).toEqual(["Night", "Night 2"]);
});

test("refuses a save with no name, past the limit, or that native does not take", async () => {
  expect(await setups.save(" \n ")).toBe("unnamed");
  for (let n = 0; n < MAX_SETUPS; n++) expect(await setups.save(`Setup ${n}`)).toBe("saved");
  expect(await setups.save("One more")).toBe("full");
  expect(setups.list()).toHaveLength(MAX_SETUPS);
  store.values["ui.setups"] = "";
  store.refuse.add("ui.setups");
  expect(await setups.save("Refused")).toBe("failed");
  expect(setups.list()).toEqual([]);
});

test("applies only what differs, and stops at the first setting that does not land", async () => {
  await setups.save("Night");
  const id = setups.list()[0]!.id;
  store.values.appearance = "light";
  store.values["ui.font"] = "serif";
  store.writes = [];
  expect(await setups.apply(id)).toBe(true);
  expect(store.writes).toEqual([
    ["appearance", "dark"],
    ["ui.font", "mono"],
  ]);

  store.values.appearance = "light";
  store.values["ui.font"] = "serif";
  store.refuse.add("appearance");
  store.writes = [];
  expect(await setups.apply(id)).toBe(false);
  expect(store.writes).toEqual([["appearance", "dark"]]);
  expect(store.values["ui.font"]).toBe("serif");
  expect(await setups.apply("0000000000000000")).toBe(false);
});

test("removes one setup and leaves the others", async () => {
  await setups.save("Night");
  await setups.save("Day");
  const [night, day] = setups.list();
  expect(await setups.remove(night!.id)).toBe(true);
  expect(setups.list()).toEqual([day]);
  expect(await setups.remove(night!.id)).toBe(false);
});

test("takes in a file as native read it, saved beside the others and not applied", async () => {
  native.import.mockResolvedValueOnce({
    outcome: "read",
    name: "",
    settings: { appearance: "light", "privacy.smartscreen": "false" },
    dropped: 2,
  });
  expect(await setups.importFile("Imported setup")).toEqual({ outcome: "read", dropped: 2 });
  const [imported] = setups.list();
  expect(imported?.name).toBe("Imported setup");
  expect(imported?.settings).toEqual({ appearance: "light" });
  // Nothing but the list was written.
  expect(store.writes.map(([key]) => key)).toEqual(["ui.setups"]);
  expect(store.values.appearance).toBe("dark");
});

test("reports a file native refused, a cancelled dialog and a call that throws", async () => {
  for (const outcome of ["cancelled", "too_large", "not_a_setup", "newer", "empty", "failed"]) {
    native.import.mockResolvedValueOnce({ outcome, name: "", settings: {}, dropped: 0 });
    expect(await setups.importFile("Imported setup")).toEqual({ outcome, dropped: 0 });
  }
  native.import.mockRejectedValueOnce(new Error("ipc"));
  expect(await setups.importFile("Imported setup")).toEqual({ outcome: "failed", dropped: 0 });
  native.import.mockResolvedValueOnce({
    outcome: "read",
    name: "Odd",
    settings: { "ai.mcp-server": "true" },
    dropped: 0,
  });
  expect((await setups.importFile("Imported setup")).outcome).toBe("empty");
  expect(setups.list()).toEqual([]);
});

test("asks native to write the file and reads a throw as a failure", async () => {
  native.export.mockResolvedValueOnce("saved");
  expect(await setups.exportFile("00000000000000aa")).toBe("saved");
  expect(native.export).toHaveBeenCalledExactlyOnceWith("00000000000000aa");
  native.export.mockRejectedValueOnce(new Error("ipc"));
  expect(await setups.exportFile("00000000000000aa")).toBe("failed");
});

test("reads the community's list from native, and an unreachable one as offline", async () => {
  const listed = {
    outcome: "listed",
    entries: [{ id: "midnight", name: "Midnight", author: "Ada", about: "Dark." }],
  };
  native.list.mockResolvedValueOnce(listed);
  expect(await setups.gallery()).toEqual(listed);
  native.list.mockRejectedValueOnce(new Error("ipc"));
  expect(await setups.gallery()).toEqual({ outcome: "offline", entries: [] });
});

test("takes a shared setup by its id, saved under its listed name and not applied", async () => {
  const entry = { id: "midnight", name: "Midnight", author: "Ada", about: "Dark." };
  native.take.mockResolvedValueOnce({
    outcome: "read",
    name: "",
    settings: { appearance: "light", "network.proxy": "http" },
    dropped: 1,
  });
  expect(await setups.takeShared(entry)).toEqual({ outcome: "read", dropped: 1 });
  expect(native.take).toHaveBeenCalledExactlyOnceWith("midnight");
  expect(setups.list().map((setup) => [setup.name, setup.settings])).toEqual([
    ["Midnight", { appearance: "light" }],
  ]);
  expect(store.values.appearance).toBe("dark");

  for (const outcome of ["offline", "unreadable", "newer"]) {
    native.take.mockResolvedValueOnce({ outcome, name: "", settings: {}, dropped: 0 });
    expect(await setups.takeShared(entry)).toEqual({ outcome, dropped: 0 });
  }
  native.take.mockRejectedValueOnce(new Error("ipc"));
  expect(await setups.takeShared(entry)).toEqual({ outcome: "offline", dropped: 0 });
  expect(setups.list()).toHaveLength(1);
});
