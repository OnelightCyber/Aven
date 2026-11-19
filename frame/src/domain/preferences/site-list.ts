/**
 * A list of sites the person set apart from a setting, as a preference keeps
 * it: one host a line. Native decides what a list covers; this only edits it
 * and says what to show.
 */

/** The site a page is on: its host without a leading `www.`, for a web page. */
export function siteOf(address: string | null | undefined): string {
  if (!address) return "";
  try {
    const parsed = new URL(address);
    if (parsed.protocol !== "https:" && parsed.protocol !== "http:") return "";
    return parsed.hostname.toLowerCase().replace(/^www\./, "");
  } catch {
    return "";
  }
}

export const sitesOf = (stored: string): string[] =>
  stored
    .split("\n")
    .map((line) => line.trim())
    .filter(Boolean);

const under = (site: string, host: string) => site === host || site.endsWith(`.${host}`);

export const covers = (stored: string, site: string): boolean =>
  site !== "" && sitesOf(stored).some((host) => under(site, host));

/** The list with `site` on it. */
export function withSite(stored: string, site: string): string {
  return site === "" || covers(stored, site) ? stored : [...sitesOf(stored), site].join("\n");
}

/** The list without whatever covers `site`. */
export function withoutSite(stored: string, site: string): string {
  return sitesOf(stored)
    .filter((host) => !under(site, host))
    .join("\n");
}
