import { commands } from "$shared/ipc/bindings";
import type { SyncCodeView, SyncOutcome, SyncStatusView } from "$shared/ipc/bindings";

export type { SyncOutcome };
export type SyncStatus = SyncStatusView;

/** Whether this build can sync at all, and whether this device does. */
export async function status(): Promise<SyncStatus> {
  try {
    return await commands.syncStatus();
  } catch {
    return { available: false, on: false };
  }
}

/** This device's code, made on first asking. Native keeps the secret; the
 *  code is shown, never stored here. */
export async function code(): Promise<SyncCodeView> {
  try {
    return await commands.syncCode();
  } catch {
    return { outcome: "failed", code: "" };
  }
}

/** Joins the devices of a code typed here. Native reads the code; text that
 *  is not one is refused there. */
export async function join(typed: string): Promise<SyncOutcome> {
  try {
    return await commands.syncJoin(typed);
  } catch {
    return "failed";
  }
}

export async function now(): Promise<SyncOutcome> {
  try {
    return await commands.syncNow();
  } catch {
    return "failed";
  }
}

/** Stops syncing on this device. Settings stay as they are. */
export async function stop(): Promise<boolean> {
  try {
    return await commands.syncStop();
  } catch {
    return false;
  }
}
