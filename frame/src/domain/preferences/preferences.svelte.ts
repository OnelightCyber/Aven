import { observe, pause } from "$shared/lib/observe";
import type { SearchShortcut } from "$shared/ipc/bindings";
import { commands } from "$shared/ipc/bindings";
import { events } from "$shared/ipc/native-events";
import { settle } from "$domain/operations";

const defaults = {
  appearance: "system",
  "search.shortcuts": "",
  "search.engine": "duckduckgo",
  "search.custom-url": "",
  "search.suggestions": "true",
  "sidebar.mode": "default",
  "ui.accent": "graphite",
  "ui.material": "auto",
  "ui.reduce-motion": "false",
  "ui.newtab-greeting": "false",
  "ui.newtab-name": "false",
  "ui.newtab-clock": "true",
  "ui.newtab-clock-format": "system",
  "ui.newtab-tasks": "true",
  "ui.newtab-background": "sky",
  "ui.newtab-image": "",
  "ui.newtab-dim": "false",
  "ui.newtab-wordmark": "true",
  "ui.newtab-trackers": "true",
  "ui.newtab-focus": "true",
  "ui.tab-layout": "vertical",
  "ai.enabled": "true",
  "ai.mcp-server": "false",
  "work.enabled": "true",
  "work.screenshots": "true",
  "ui.language": "system",
  "ui.density": "comfortable",
  "ui.contrast": "false",
  "ui.force-dark": "false",
  "search.history": "true",
  "tabs.new-position": "end",
  "tabs.after-close": "next",
  "tabs.switch-to-open": "true",
  "tabs.automatic-pip": "true",
  "tabs.startup": "continue",
  "tabs.startup-pages": "",
  "tabs.home": "",
  "network.proxy": "system",
  "network.proxy-address": "",
  "time.track": "true",
  "time.retention": "90",
  "focus.minutes": "25",
  "focus.breaks": "false",
  "focus.goal": "120",
  "focus.blocked": "",
  "performance.sleep": "true",
  "performance.after": "30",
  "performance.memory": "balanced",
  "performance.game-mode": "true",
  "performance.memory-cap": "0",
  "performance.cpu-cap": "0",
  "performance.network-cap": "0",
  "performance.network-cap-when": "game",
  "performance.exceptions": "",
  "updates.auto-check": "true",
  "ui.theme": "aven",
  "ui.theme-custom": "",
  "ui.window-tint": "0",
  "ui.font": "system",
  "ui.font-size": "13",
  "ui.code-font": "system",
  "ui.theme-contrast": "50",
  "tabs.archive": "never",
  "tabs.external-position": "new-tab",
  "tabs.close-button": "hover",
  "notes.enabled": "true",
  "tasks.enabled": "true",
  "time.enabled": "true",
  "sidebar.apps": "true",
  "work.favourite-models": "",
  "zoom.default": "100",
  "privacy.smartscreen": "true",
  "privacy.cookie-banners": "true",
  "privacy.streamer-mode": "false",
  "privacy.streamer-auto": "true",
  "privacy.cookie-banners-except": "",
  "ui.force-dark-except": "",
  "ui.setups": "",
} as const;
export type PreferenceKey = keyof typeof defaults;
const STARTUP_CONCURRENCY = 8;
const STARTUP_TIMEOUT_MS = 2000;
let shortcutDefaults = $state.raw<SearchShortcut[]>([]);
const values = $state<Record<PreferenceKey, string>>({ ...defaults });
let pending = $state<PreferenceKey | null>(null);
let failed = $state(false);
let epoch = 0;
let stop: (() => void) | null = null;
let initialized = false;
let initializing: Promise<void> | null = null;
let lifetime: AbortController | null = null;
const versions: Partial<Record<PreferenceKey, number>> = {};
const shortcuts = $derived.by(() => {
  const stored = values["search.shortcuts"];
  if (!stored) return shortcutDefaults;
  try {
    return JSON.parse(stored) as SearchShortcut[];
  } catch {
    return shortcutDefaults;
  }
});
export const searchShortcuts = () => shortcuts;
export const value = (key: PreferenceKey) => values[key];
export const saving = () => pending !== null;
export const saveFailed = () => failed;
/** AI: models, agents and dictation. Off, no AI surface is shown. */
export const aiEnabled = () => values["ai.enabled"] !== "false";
export type Feature = "work" | "notes" | "tasks" | "time";
/** A feature switch: off, the feature leaves the app; its data stays. */
export const enabled = (feature: Feature) => values[`${feature}.enabled`] !== "false";
/** The feature a command belongs to. Mirrors `aven_core::commands::feature_of`. */
function featureOf(command: string): Feature | "ai" | null {
  if (["work.dictate", "tool.ai", "mode.choose"].includes(command)) return "ai";
  if (command === "browser.work" || command === "mode.work" || command.startsWith("work."))
    return "work";
  if (["tool.notes", "browser.notes", "note.new"].includes(command)) return "notes";
  if (command === "tool.tasks" || command === "browser.tasks") return "tasks";
  if (
    ["tool.time", "browser.time", "settings.focus"].includes(command) ||
    command.startsWith("focus.")
  )
    return "time";
  return null;
}
/** Whether a command's feature is on, so it is offered at all. */
export function commandOffered(command: string) {
  const feature = featureOf(command);
  return feature === null || (feature === "ai" ? aiEnabled() : enabled(feature));
}
function apply(key: PreferenceKey, next: string) {
  values[key] = next;
  versions[key] = (versions[key] ?? 0) + 1;
}
async function initialize(generation: number, signal: AbortSignal) {
  const updated: Partial<Record<PreferenceKey, true>> = {};
  const unsubscribe = await events.uiCommand.listen(({ payload }) => {
    if (generation !== epoch) return;
    if (payload.startsWith("theme.")) {
      updated.appearance = true;
      apply("appearance", payload.slice(6));
    }
    if (!payload.startsWith("preference.")) return;
    const separator = payload.indexOf("=", 11);
    if (separator < 0) return;
    const key = payload.slice(11, separator);
    const next = payload.slice(separator + 1);
    if (key === undefined || !Object.hasOwn(defaults, key) || next === undefined) return;
    updated[key as PreferenceKey] = true;
    apply(key as PreferenceKey, next);
  });
  if (generation !== epoch) {
    unsubscribe();
    return;
  }
  stop = unsubscribe;
  const keys = Object.keys(defaults) as PreferenceKey[];
  // An observation timeout leaves its native request in flight. Every read
  // shares the same deadline and stops after a timeout.
  const deadline = performance.now() + STARTUP_TIMEOUT_MS;
  // Every setting in one answer: the window waits for this before it shows,
  // and one call is back long before eighty are. False when native has no
  // such answer, so each setting is read on its own instead.
  async function readAll(): Promise<boolean> {
    try {
      const all = await observe(commands.settingsSnapshot(), deadline - performance.now(), signal);
      if (all.state === "timeout") return true;
      if (generation !== epoch || all.state !== "received") return false;
      for (const key of keys) if (!updated[key]) apply(key, all.value[key] ?? defaults[key]);
      return true;
    } catch {
      return false;
    }
  }
  // With that answer in hand only the shortcut catalogue is left to read.
  const reads: readonly ("shortcutCatalogue" | PreferenceKey)[] = (await readAll())
    ? ["shortcutCatalogue"]
    : ["shortcutCatalogue", ...keys];
  let next = 0;
  async function read() {
    while (generation === epoch && !signal.aborted) {
      const remaining = deadline - performance.now();
      if (remaining <= 0) return;
      const key = reads[next++];
      if (key === undefined) return;
      try {
        if (key === "shortcutCatalogue") {
          const result = await observe(commands.searchShortcutDefaults(), remaining, signal);
          if (result.state === "timeout") return;
          if (generation === epoch && result.state === "received") shortcutDefaults = result.value;
        } else if (!updated[key]) {
          const stored = await observe(commands.settingGet(key), remaining, signal);
          if (stored.state === "timeout") return;
          if (generation === epoch && !updated[key] && stored.state === "received")
            apply(key, stored.value ?? defaults[key]);
        }
      } catch {
        /* Retain safe defaults if a read is unavailable. */
      }
    }
  }
  await Promise.all(Array.from({ length: STARTUP_CONCURRENCY }, read));
  if (generation === epoch) initialized = true;
}

export function init(): Promise<void> {
  if (initializing) return initializing;
  if (initialized) return Promise.resolve();
  lifetime = new AbortController();
  const generation = ++epoch;
  const task = initialize(generation, lifetime.signal);
  initializing = task;
  void task.then(
    () => {
      if (initializing === task) initializing = null;
    },
    () => {
      if (generation === epoch) {
        stop?.();
        stop = null;
        lifetime?.abort();
      }
      if (initializing === task) initializing = null;
    },
  );
  return task;
}
export function dispose() {
  epoch++;
  lifetime?.abort();
  lifetime = null;
  initializing = null;
  initialized = false;
  stop?.();
  stop = null;
  pending = null;
  shortcutDefaults = [];
  failed = false;
  for (const key of Object.keys(defaults) as PreferenceKey[]) {
    values[key] = defaults[key];
    delete versions[key];
  }
}
export async function set(key: PreferenceKey, next: string) {
  if (!initialized) {
    failed = true;
    return;
  }
  if (pending !== null || values[key] === next) return;
  const generation = epoch;
  const signal = lifetime?.signal;
  pending = key;
  failed = false;
  try {
    const result = await settle(commands.settingSet(key, next), 5000, signal);
    if (result.outcome === "failed" || result.outcome === "rejected")
      throw new Error("not processed");
    // Queue admission is not durable success. Re-read through the store before
    // ending the pending presentation; never display a fabricated Saved toast.
    let observed = false;
    const deadline = performance.now() + 1500;
    while (generation === epoch && !signal?.aborted) {
      const remaining = deadline - performance.now();
      if (remaining <= 0) break;
      const version = versions[key];
      const stored = await observe(commands.settingGet(key), remaining, signal);
      if (generation !== epoch || signal?.aborted) return;
      if (stored.state === "received" && stored.value === next) {
        observed = true;
        if (versions[key] === version) apply(key, next);
        break;
      }
      if (stored.state === "timeout" || stored.state === "aborted") break;
      await pause(Math.min(100, Math.max(0, deadline - performance.now())), signal);
    }
    if (!observed) throw new Error("readback unavailable");
  } catch {
    if (generation === epoch) failed = true;
  } finally {
    if (generation === epoch) pending = null;
  }
}

/** What a reset leaves alone: things the person made, kept in a setting. */
const KEPT_ON_RESET: readonly PreferenceKey[] = ["ui.setups"];

/** Every preference back to how Aven ships, one validated write at a
 *  time. False when any of them did not land. The saved setups stay. */
export async function resetAll(): Promise<boolean> {
  for (const key of Object.keys(defaults) as PreferenceKey[]) {
    if (KEPT_ON_RESET.includes(key) || values[key] === defaults[key]) continue;
    await set(key, defaults[key]);
    if (failed || values[key] !== defaults[key]) return false;
  }
  return true;
}

/** Reset just this page, including choices whose feature is currently off. */
export async function resetNewTab(): Promise<boolean> {
  for (const key of Object.keys(defaults) as PreferenceKey[]) {
    if (!key.startsWith("ui.newtab-") || values[key] === defaults[key]) continue;
    await set(key, defaults[key]);
    if (failed || values[key] !== defaults[key]) return false;
  }
  return true;
}
