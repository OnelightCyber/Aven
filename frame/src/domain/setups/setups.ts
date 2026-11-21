import { preferences } from "$domain/preferences";
import { commands } from "$shared/ipc/bindings";
import type {
  SetupExportView,
  SetupGalleryEntryView,
  SetupGalleryView,
  SetupImportOutcome,
} from "$shared/ipc/bindings";
import * as model from "./model";

const read = (key: model.SetupKey) => preferences.value(key);

/** The saved setups, in the order they were saved. */
export const list = (): model.Setup[] => model.parse(preferences.value("ui.setups"));
export const inUse = (setup: model.Setup) => model.inUse(setup, read);
export const busy = () => preferences.saving();

/** The list is one setting: native takes it whole or leaves the old one. */
async function store(next: readonly model.Setup[]): Promise<boolean> {
  const value = model.serialize(next);
  await preferences.set("ui.setups", value);
  return !preferences.saveFailed() && preferences.value("ui.setups") === value;
}

export type SaveOutcome = "saved" | "unnamed" | "full" | "failed";

/** Saves how Aven is set up now under a name. */
export async function save(name: string): Promise<SaveOutcome> {
  const clean = model.cleanName(name);
  if (!clean) return "unnamed";
  const current = list();
  if (current.length >= model.MAX_SETUPS) return "full";
  const setup: model.Setup = {
    id: model.newId(),
    name: model.uniqueName(
      clean,
      current.map((saved) => saved.name),
    ),
    settings: model.snapshot(read),
  };
  return (await store([...current, setup])) ? "saved" : "failed";
}

export async function remove(id: string): Promise<boolean> {
  const current = list();
  const next = current.filter((setup) => setup.id !== id);
  return next.length === current.length ? false : store(next);
}

/** Applies a setup one validated setting at a time, as a hand would. False
 *  when one did not land: those before it stay, the rest are not tried. */
export async function apply(id: string): Promise<boolean> {
  const setup = list().find((saved) => saved.id === id);
  if (!setup) return false;
  for (const [key, value] of model.changes(setup, read)) {
    await preferences.set(key, value);
    if (preferences.saveFailed() || preferences.value(key) !== value) return false;
  }
  return true;
}

export type ExportOutcome = SetupExportView;

/** Native writes the setup the store holds to a file the person chooses. */
export async function exportFile(id: string): Promise<ExportOutcome> {
  try {
    return await commands.setupsExport(id);
  } catch {
    return "failed";
  }
}

export type ImportOutcome = {
  outcome: SetupImportOutcome | "full" | "offline" | "unreadable";
  /** Entries a setup may not hold, left out by native. */
  dropped: number;
};

/** Saves what native read from a file or from the community beside the
 *  others. It is applied only when asked. */
async function keep(
  name: string,
  read: unknown,
  dropped: number,
  fallbackName: string,
): Promise<ImportOutcome> {
  const current = list();
  if (current.length >= model.MAX_SETUPS) return { outcome: "full", dropped: 0 };
  const settings = model.admit(read);
  if (Object.keys(settings).length === 0) return { outcome: "empty", dropped };
  const setup: model.Setup = {
    id: model.newId(),
    name: model.uniqueName(
      model.cleanName(name) || model.cleanName(fallbackName),
      current.map((saved) => saved.name),
    ),
    settings,
  };
  return { outcome: (await store([...current, setup])) ? "read" : "failed", dropped };
}

/** Native reads a file as untrusted data and returns only what a setup may
 *  hold. */
export async function importFile(fallbackName: string): Promise<ImportOutcome> {
  let view;
  try {
    view = await commands.setupsImport();
  } catch {
    return { outcome: "failed", dropped: 0 };
  }
  if (view.outcome !== "read") return { outcome: view.outcome, dropped: 0 };
  return keep(view.name, view.settings, view.dropped, fallbackName);
}

export type GalleryEntry = SetupGalleryEntryView;
export type Gallery = SetupGalleryView;

/** The community's list, fetched from Aven's public repository. Asked for
 *  only when the person opens it. */
export async function gallery(): Promise<Gallery> {
  try {
    return await commands.setupsGalleryList();
  } catch {
    return { outcome: "offline", entries: [] };
  }
}

/** Takes one shared setup: native fetches its file and reads it as it reads
 *  any file. */
export async function takeShared(entry: GalleryEntry): Promise<ImportOutcome> {
  let view;
  try {
    view = await commands.setupsGalleryTake(entry.id);
  } catch {
    return { outcome: "offline", dropped: 0 };
  }
  if (view.outcome !== "read") return { outcome: view.outcome, dropped: 0 };
  return keep(view.name, view.settings, view.dropped, entry.name);
}
