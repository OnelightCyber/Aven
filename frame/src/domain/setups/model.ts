import type { PreferenceKey } from "$domain/preferences";

/** The settings a setup carries. Mirrors `aven_core::setups::KEYS`, which is
 *  what native admits; a test holds the two lists together. */
export const SETUP_KEYS = [
  "appearance",
  "ui.theme",
  "ui.theme-custom",
  "ui.theme-contrast",
  "ui.accent",
  "ui.material",
  "ui.window-tint",
  "ui.font",
  "ui.font-size",
  "ui.code-font",
  "ui.density",
  "ui.contrast",
  "ui.reduce-motion",
  "ui.force-dark",
  "ui.tab-layout",
  "sidebar.mode",
  "sidebar.apps",
  "tabs.close-button",
  "tabs.new-position",
  "tabs.after-close",
  "ui.newtab-greeting",
  "ui.newtab-clock",
  "ui.newtab-clock-format",
  "ui.newtab-tasks",
  "ui.newtab-dim",
  "ui.newtab-wordmark",
  "ui.newtab-trackers",
  "ui.newtab-focus",
  "notes.enabled",
  "tasks.enabled",
  "time.enabled",
] as const satisfies readonly PreferenceKey[];
export type SetupKey = (typeof SETUP_KEYS)[number];

export const MAX_SETUPS = 12;
export const MAX_NAME_CHARS = 40;

export type SetupSettings = Partial<Record<SetupKey, string>>;
export type Setup = { id: string; name: string; settings: SetupSettings };
type Read = (key: SetupKey) => string;

const isKey = (key: string): key is SetupKey => (SETUP_KEYS as readonly string[]).includes(key);
const ID = /^[0-9a-f]{16}$/u;

/** Only the entries a setup carries, as text. Native has the last word on
 *  values; this keeps anything else from being shown or sent. */
export function admit(settings: unknown): SetupSettings {
  const kept: SetupSettings = {};
  if (typeof settings !== "object" || settings === null) return kept;
  for (const [key, value] of Object.entries(settings)) {
    if (isKey(key) && typeof value === "string") kept[key] = value;
  }
  return kept;
}

/** The saved list as the setting stores it. Native admits that list whole,
 *  so one this cannot read is shown as no setups rather than half of them. */
export function parse(stored: string): Setup[] {
  if (!stored) return [];
  let list: unknown;
  try {
    list = JSON.parse(stored);
  } catch {
    return [];
  }
  if (!Array.isArray(list)) return [];
  const setups: Setup[] = [];
  for (const entry of list as unknown[]) {
    if (typeof entry !== "object" || entry === null) return [];
    const { id, name, settings } = entry as Record<string, unknown>;
    if (typeof id !== "string" || !ID.test(id) || typeof name !== "string" || !name) return [];
    setups.push({ id, name, settings: admit(settings) });
  }
  return setups;
}

/** No setups is the empty setting, as on a fresh install. */
export function serialize(setups: readonly Setup[]): string {
  return setups.length === 0 ? "" : JSON.stringify(setups);
}

/** A name as native takes it: printable, trimmed, at most forty characters. */
export function cleanName(name: string): string {
  const printable = [...name].filter((c) => !/\p{Cc}/u.test(c)).join("");
  return [...printable.trim()].slice(0, MAX_NAME_CHARS).join("").trim();
}

/** The name, or the name with the first number that sets it apart. */
export function uniqueName(name: string, taken: readonly string[]): string {
  if (!taken.includes(name)) return name;
  for (let n = 2; ; n++) {
    const suffix = ` ${n}`;
    const candidate =
      [...name]
        .slice(0, MAX_NAME_CHARS - suffix.length)
        .join("")
        .trim() + suffix;
    if (!taken.includes(candidate)) return candidate;
  }
}

export function newId(random: Uint8Array = crypto.getRandomValues(new Uint8Array(8))): string {
  return [...random.slice(0, 8)].map((byte) => byte.toString(16).padStart(2, "0")).join("");
}

/** Every setting a setup carries, as it is now. */
export function snapshot(read: Read): SetupSettings {
  const settings: SetupSettings = {};
  for (const key of SETUP_KEYS) settings[key] = read(key);
  return settings;
}

/** What applying the setup would change, in the order it would change it. */
export function changes(setup: Setup, read: Read): [SetupKey, string][] {
  const pending: [SetupKey, string][] = [];
  for (const key of SETUP_KEYS) {
    const value = setup.settings[key];
    if (value !== undefined && value !== read(key)) pending.push([key, value]);
  }
  return pending;
}

/** Whether Aven is set up this way now. A setup holding nothing is never
 *  in use. */
export function inUse(setup: Setup, read: Read): boolean {
  return Object.keys(setup.settings).length > 0 && changes(setup, read).length === 0;
}
