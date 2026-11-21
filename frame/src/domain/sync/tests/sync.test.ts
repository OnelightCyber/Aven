import { beforeEach, expect, test, vi } from "vitest";

const native = vi.hoisted(() => ({
  status: vi.fn(),
  code: vi.fn(),
  join: vi.fn(),
  now: vi.fn(),
  stop: vi.fn(),
}));
vi.mock("$shared/ipc/bindings", async () => {
  const { mockBindings } = await import("$shared/testing/bindings");
  return mockBindings({
    syncStatus: native.status as never,
    syncCode: native.code as never,
    syncJoin: native.join as never,
    syncNow: native.now as never,
    syncStop: native.stop as never,
  });
});

import * as sync from "..";

beforeEach(() => {
  for (const mock of Object.values(native)) mock.mockReset();
});

test("returns what native answers", async () => {
  native.status.mockResolvedValueOnce({ available: true, on: true });
  expect(await sync.status()).toEqual({ available: true, on: true });
  native.code.mockResolvedValueOnce({ outcome: "synced", code: "7K2M-0000" });
  expect(await sync.code()).toEqual({ outcome: "synced", code: "7K2M-0000" });
  native.join.mockResolvedValueOnce("refused");
  expect(await sync.join("not a code")).toBe("refused");
  expect(native.join).toHaveBeenCalledExactlyOnceWith("not a code");
  native.now.mockResolvedValueOnce("offline");
  expect(await sync.now()).toBe("offline");
  native.stop.mockResolvedValueOnce(true);
  expect(await sync.stop()).toBe(true);
});

test("a call that throws reads as unavailable, failed or not stopped, never as done", async () => {
  for (const mock of Object.values(native)) mock.mockRejectedValueOnce(new Error("ipc"));
  expect(await sync.status()).toEqual({ available: false, on: false });
  expect(await sync.code()).toEqual({ outcome: "failed", code: "" });
  expect(await sync.join("7K2M")).toBe("failed");
  expect(await sync.now()).toBe("failed");
  expect(await sync.stop()).toBe(false);
});
