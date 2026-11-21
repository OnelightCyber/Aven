# Changelog

Notable changes to Aven. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow
[semantic versioning](https://semver.org/).

## Unreleased

### Added

- Setups: save how Aven looks and is laid out under a name, switch between
  setups, and carry one to another machine as a file (Settings, Appearance).
  A setup changes appearance and layout only; a file from someone else
  cannot touch privacy, security, search or your data, and is saved rather
  than applied.
- Community setups: a gallery read from this repository when you open it,
  with six setups to start from. Sharing one is a pull request
  ([community/setups](community/setups/README.md)).
- A relay for encrypted sync and for working together, as a Cloudflare
  Worker with no dependencies ([relay](relay/README.md)). It stores and
  passes on only what it cannot read.
- Sync between your devices, with no account: appearance, setups, language
  and search engine, encrypted on the device with a key derived from a code
  you type on the other device (Settings, General). The relay never sees
  the code, the key or anything readable.
- A [roadmap](ROADMAP.md).

## 1.0.6 - 2026-11-20

### Changed

- The installer and the program's file properties name Aven alone. Where
  the code comes from stays in [NOTICE.md](NOTICE.md).
- The interface does less work each time something changes. Reduced motion
  and game mode cost nothing while they are off, the name on the new tab is
  one shape instead of several hundred, and opening a panel, a tab or a
  settings page no longer has styles checked again across the whole window.

### Fixed

- The repository's own checks pass again: tests still held values from
  before the code was renamed, and the page reader's boundary check did not
  know the two recipes of the agent's pointer.

## 1.0.5 - 2026-11-19

### Changed

- Aven starts again from a new code base under the MPL-2.0; where it comes
  from and who to credit is in [NOTICE.md](NOTICE.md). Everything is named
  Aven, with its own logo. Accounts and hosted models, which depended on
  servers Aven does not have, are off.
- A new window opens on the tabs you left, instead of a new tab.
- The window's minimize, maximize and close controls are drawn by Aven itself
  and stay inside the window, with pages placed below them.

### Added

**Privacy**

- A vault for sign-ins, notes and two-factor codes, encrypted on disk, with
  its key kept by Windows. It imports the CSV file another browser exports.
- Saved sign-ins filled from the site menu, only on the encrypted site each
  was saved for, and the sign-in typed on a page saved from the same menu.
- Copied passwords and codes stay out of Windows' clipboard history and
  leave the clipboard after 45 seconds.
- A deleted profile takes its vault and the vault's key with it.
- Cookie banners refused automatically when the site offers a refusal, except
  on the sites you leave out.
- Streamer mode, which blurs your e-mail and the vault's contents in the
  interface while you share your screen. It starts by itself while OBS,
  Streamlabs, XSplit or a similar program runs, and holds Aven's
  notifications back.

**Play**

- Game mode: when a game is in front, hidden tabs sleep and Aven runs at low
  priority.
- Limits on memory and processor use, and a network speed limit that applies
  while you play or all the time.
- A task manager that shows what each tab costs and puts a tab to sleep.
- Discord, Twitch, WhatsApp and Spotify in the tools menu, opened beside the
  page in a split.

**Install and update**

- The installer speaks your computer's language, among the thirteen Aven has,
  and so does Aven on its first launch.
- Aven updates itself from its own signed releases on GitHub. A release whose
  signature does not match the key built into the app is never installed. The
  check can be turned off in Settings.

**Pages**

- A dark theme forced on sites that only come in light, except on the sites
  you leave out.
- A volume for each tab, in the site menu.
- The window's size and place are remembered between launches.

**Agents**

- Ask Aven can be sent a picture of what the page shows, of the whole page or
  of your screen, with a model that takes pictures.
- Auto mode in Ask Aven: give it a mission and it searches the web, opens
  and reads pages and acts on them on its own, stopping only for a CAPTCHA,
  a sign-in, a payment or a step that sends, buys or deletes. When Ask Aven
  has a question the open pages do not answer, it looks it up itself.
- Ask Aven is one press away at the foot of the sidebar.
- When Ask Aven acts on a page, a pointer shows where it clicks and types,
  with "Aven is working" on the page. It closes cookie banners and boxes in
  its way by themselves; a CAPTCHA is left to you.
- An MCP server, off until you turn it on, that lets an agent on your computer
  list regular tabs, read a page, picture it and open a tab. It presses, types,
  chooses and ticks on a page only with your yes, asked in the sidebar for
  one action or for the site, and you can stop it at any time. It never sees
  private windows or password, card and code fields.
- Models of a Claude Code install on the same computer, through a local
  endpoint that answers only with its secret.

### Fixed

- Interface texts added for the vault, the task manager, game mode, the
  Claude Code subscription, skills, Windows privacy and site search were in
  English in every language but French; every language is now complete.
- The steps an agent takes were named in English whatever the language.
- A search box that suggests as you type, such as DuckDuckGo's, was read as
  a list, so an agent could not type into it.
- With a Claude Code model, the panel could show raw tool calls or a
  model's own corrections instead of its answer.
- Claude Code models were listed by their identifiers and given a smaller
  context than they read; they show their names and read 200,000 tokens.
- Game mode took any maximized window for a game when the taskbar hides
  itself, and put tabs to sleep while you worked.
- The model card's "own pace" label lay over the model's name, and seven
  components took a stylesheet utility's layout by mistake.
- The hint where kept tabs go took three lines in several languages, and
  the field to keep a site awake was too narrow to read.
- The tab volume showed a level the page had not taken.
- "Put to sleep" in the task manager did nothing.
- A web page covered the lower part of the window controls.
- The Time panel overflowed in some languages; several labels were not
  translated.

### Security

- The vault opens only for the profile the window browses as and never in
  a private window, and looking at a vault that holds nothing no longer
  makes a key in the Credential Manager.
- What the vault holds is wiped from memory once read, a password shown
  hides again after 30 seconds or when the window is put away, and
  generated passwords are drawn and shuffled without bias.
- Both local endpoints turn away a caller without their secret before
  reading what it sends; the MCP server takes 1 MiB per message at most.
- Turning the MCP server off ends its connections and takes back every
  yes given for a site; its address, which holds its secret, is copied out
  of the clipboard history as a password is.
- A label never names a site from what an address carries before an `@`,
  and a search engine's redirect is named by its destination only when
  every address it holds agrees.
- Microsoft announced on 17 November a WebView2 security fix still to
  come. WebView2 updates itself through Windows; the next release of Aven
  will require the fixed runtime.
- The oldest WebView2 runtime Aven treats as up to date is now
  157.0.4341.46, which fixes a flaw Microsoft lists as exploited in the wild.
- The local Claude Code endpoint answers only a caller that holds its secret,
  no longer runs tools that could read files, and keeps page text from posing
  as the person's own message.
- The vault is written to disk safely, keeps the previous version beside it,
  and is never given a new key over an existing file.
- Game mode no longer slows a program that is not Aven's.
