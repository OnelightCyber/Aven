import * as m from "$shared/i18n/messages";

/** The core names a new tab, the first space and the first profile in
 *  English; the interface shows them in its own language. A name the person
 *  chose is left alone. */
const NEW_TAB = "New Tab";
const DEFAULT_SPACE = "Space";
const DEFAULT_PROFILE = "Personal";

export const tabTitle = (title: string): string => (title === NEW_TAB ? m.new_tab() : title);

export const spaceName = (name: string): string =>
  name === DEFAULT_SPACE ? m.space_default_name() : name;

export const profileName = (name: string): string =>
  name === DEFAULT_PROFILE ? m.profile_default_name() : name;

/** The profile's name only when the person chose it: a greeting does not
 *  call them by the name every first profile starts with. */
export const chosenProfileName = (name: string): string => (name === DEFAULT_PROFILE ? "" : name);
