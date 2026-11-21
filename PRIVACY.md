# Privacy

Aven keeps what you do in it on your computer. This page lists every time
Aven itself, rather than a site you visit, connects to something, what it
sends, and how to turn it off.

## What Aven connects to on its own

| What | Where | What it sends | When | Turn it off |
|---|---|---|---|---|
| **Update check** | GitHub (`github.com/OnelightCyber/Aven`) | An ordinary download request for the release manifest. | About once a day. | Settings, About: automatic update checks. |
| **Ad and tracker lists** | `easylist.to`, `ublockorigin.github.io` | An ordinary download request for each list, asking only whether it changed. | About once a day while the blocker is on. | Settings, Privacy & security: turn the blocker off. |
| **Microsoft Defender SmartScreen** | Microsoft | The addresses of the sites you open and information about your downloads, for Microsoft to check against phishing and malware. | While you browse, if it is on. It is on unless you turn it off. | On the welcome screen at first launch, where it is shown before anything else, or later in Settings, Privacy & security (from the next launch). |
| **Search suggestions** | The search engine you chose | What you type in the address field, as you type it. | While you type, if it is on. | Settings, Search: search suggestions. |

## What Aven connects to only when you ask

| What | Where | What it sends |
|---|---|---|
| **Sync between your devices** | Aven's relay on Cloudflare (`aven-relay.aven-browser.workers.dev`) | Your synced settings, encrypted on your device with a key made from a code only your devices know. The relay stores the encrypted box and cannot read it. Nothing is sent until you turn sync on. |
| **Community setups** | GitHub (`raw.githubusercontent.com`) | An ordinary download request for the list, when you open the gallery, and for a setup, when you take it. |
| **Model providers** | The providers you add, with your own keys | What you ask them, and the page you ask about when you ask about a page. |
| **Extensions** | The Chrome Web Store | An ordinary download request for an extension you install. |

## What Aven never does

- It has no account to create and no server that receives your browsing.
- It sends no analytics, usage statistics or crash reports.
- It keeps your passwords, history, bookmarks and settings on your
  computer. Passwords are encrypted with AES-256-GCM; the key stays in the
  Windows Credential Manager.

## The web engine

Pages are drawn by Microsoft Edge WebView2, which is part of Windows. Microsoft
keeps it up to date through its own update service. Any diagnostic or crash
data it reports to Microsoft follows the diagnostic data settings of Windows,
not Aven's. Aven gives it no information of its own.

## The sites you visit

A site you open receives what any browser sends it. The blocker stops known
ads and trackers; cookie banners are answered with a refusal where Aven can.

## Questions

Ask in an issue on [GitHub](https://github.com/OnelightCyber/Aven/issues).
For a security problem, see [SECURITY.md](docs/SECURITY.md).
