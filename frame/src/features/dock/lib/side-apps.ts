import { DiscordIcon, SpotifyIcon, TwitchIcon, WhatsappIcon } from "@hugeicons/core-free-icons";

/** A web app kept one click away in the tools menu. Names are the services'
 *  own and are not translated. */
export type SideApp = {
  id: string;
  name: string;
  /** Where it opens. */
  url: string;
  /** The hosts its tabs live on, to find one already open. */
  hosts: readonly string[];
  icon: typeof DiscordIcon;
};

export const SIDE_APPS: readonly SideApp[] = [
  {
    id: "discord",
    name: "Discord",
    url: "https://discord.com/app",
    hosts: ["discord.com"],
    icon: DiscordIcon,
  },
  {
    id: "twitch",
    name: "Twitch",
    url: "https://www.twitch.tv/",
    hosts: ["www.twitch.tv", "twitch.tv"],
    icon: TwitchIcon,
  },
  {
    id: "whatsapp",
    name: "WhatsApp",
    url: "https://web.whatsapp.com/",
    hosts: ["web.whatsapp.com"],
    icon: WhatsappIcon,
  },
  {
    id: "spotify",
    name: "Spotify",
    url: "https://open.spotify.com/",
    hosts: ["open.spotify.com"],
    icon: SpotifyIcon,
  },
];

const hostOf = (address: string | null): string => {
  if (!address) return "";
  try {
    return new URL(address).hostname;
  } catch {
    return "";
  }
};

/** The tab already showing `app`, if one is open. */
export function openTabOf<Tab extends { id: string; url: string | null }>(
  app: SideApp,
  tabs: readonly Tab[],
): Tab | null {
  return tabs.find((tab) => app.hosts.includes(hostOf(tab.url))) ?? null;
}

/** What opening `app` asks of the browser: nothing when it is already in
 *  front, a split with its open tab, or a new tab split with the page that
 *  was in front. */
export function sideAppPlan<Tab extends { id: string; url: string | null }>(
  app: SideApp,
  tabs: readonly Tab[],
  active: string | null,
): { open: string | null; splitWith: string | null } {
  const existing = openTabOf(app, tabs);
  if (existing) return { open: null, splitWith: existing.id === active ? null : existing.id };
  return { open: app.url, splitWith: active };
}
