# Roadmap

Where Aven is going. Nothing here is a promise or a date: it is the order
things are being built in, and it changes as people use Aven and say what is
missing. What already ships is in the [README](README.md) and the
[changelog](CHANGELOG.md).

To weigh in, open an issue or a discussion. To build one of these, say so on
its issue first so two people do not do it twice.

Sizes are rough: **S** is days, **M** a week or two, **L** several weeks.

## Built, in the next release

| Feature          | What it gives you                                                                                    |
| ---------------- | ---------------------------------------------------------------------------------------------------- |
| Setups           | Save how Aven looks and is laid out, switch between setups, carry one as a file                      |
| Community setups | A gallery read from this repository, with no server; sharing one is a pull request                   |
| Encrypted sync   | The same appearance, setups, language and search engine on your machines, with no account            |
| The relay        | A Cloudflare Worker ([`relay/`](relay/README.md)) that stores and passes on only what it cannot read |

## Every day

What makes a browser the one you keep.

| Feature                           | What it gives you                                                              | Size |
| --------------------------------- | ------------------------------------------------------------------------------ | ---- |
| An offer to save a password       | No trip to the site menu; never saved without a yes                            | M    |
| A generator at sign-up            | A strong password offered in the field when you make an account                | S    |
| Weak and reused password warnings | Worked out on the device; nothing is sent                                      | S    |
| Bookmarks in sync                 | Bookmarks follow you, encrypted                                                | M    |
| Send a tab to another device      | One click and it opens on your other machine, through the relay                | M    |
| Panic key                         | One shortcut hides every window and mutes every tab, then brings them back     | S    |
| Ask from the address bar          | Type `>` and a task; the agent starts on the page, with consent before it acts | M    |
| Tidy my tabs                      | Tabs sorted into groups by subject, shown before anything changes              | M    |
| Search every tab and your history | Find a page by a word in it, from an index kept on the device                  | M    |
| Reader mode                       | The page without the noise, in the type and width you choose                   | M    |
| Whole-page capture                | Capture, mark up, copy or save                                                 | S    |
| Translate the page                | With the model you already connected                                           | M    |

## Play and streaming

What sets Aven apart from a general browser.

| Feature                  | What it gives you                                                                         | Size |
| ------------------------ | ----------------------------------------------------------------------------------------- | ---- |
| Floating window          | A tab (a guide, a wiki, a video, a chat) pinned over the game, as see-through as you like | M    |
| Game mode profiles       | Different memory, processor and network limits for different games                        | M    |
| Heaviest tabs            | The three that cost the most at a glance, asleep in one click                             | S    |
| Live alerts              | A notice when a channel you follow goes live                                              | M    |
| Stream and call controls | Mute, restart and volume from the sidebar                                                 | S    |
| Living themes            | Moving backgrounds, interface sounds, shapes; off by themselves in game mode              | L    |
| A wider streamer mode    | Bookmarks, history and suggestions hidden too while you share your screen                 | S    |

## Agents

| Feature                       | What it gives you                                                               | Size |
| ----------------------------- | ------------------------------------------------------------------------------- | ---- |
| Routines                      | A task an agent repeats on a schedule, with a log of each run                   | L    |
| Do again what I just did      | Record a series of actions and replay it on another page                        | L    |
| Local models                  | Ollama and other local servers set up from Settings; nothing leaves the machine | M    |
| One-click summary             | A long article or a video summed up in the panel, with its sources              | S    |
| More for outside agents (MCP) | Downloads, bookmarks, and a session scoped to chosen tabs                       | M    |
| Compare                       | The agent compares a product across sites and returns a table                   | M    |

## Trust and reach

What makes people willing to install it, and able to find it.

| Feature                               | What it gives you                                    | Size |
| ------------------------------------- | ---------------------------------------------------- | ---- |
| A signed installer                    | Windows stops warning about an unknown publisher     | M    |
| `winget install` and a portable build | Install in one command, or not at all                | S    |
| A beta channel                        | Volunteers try a release before everyone             | S    |
| Privacy report                        | What was blocked, for which sites, over time         | M    |
| Permissions at a glance               | See and revoke what each site may do                 | M    |
| Encrypted profile export              | Move a whole profile, or keep a backup               | M    |
| The vault behind Windows Hello        | A fingerprint or a PIN before a password is shown    | M    |
| Breach check                          | Learn that a password leaked without ever sending it | M    |
| Encrypted DNS of your choice          | Pick a resolver from Settings                        | S    |
| A site and documentation              | A clear page, pictures, the download                 | M    |
| The gallery on the web                | Browse setups without opening Aven                   | S    |

## Together

| Feature                       | What it gives you                                                             | Size |
| ----------------------------- | ----------------------------------------------------------------------------- | ---- |
| Working together on a canvas  | Several people see and edit the same Work canvas live, encrypted between them | L    |
| Watch together                | Share a tab to watch in step, by a link                                       | L    |
| History and open tabs in sync | Pick up on one machine what you were doing on another                         | L    |
| macOS checked and released    | The code is there; it needs a real Mac                                        | L    |
| Linux                         | Looked at seriously                                                           | L    |

## Always

- A startup budget, measured at each release.
- Settings pages that switch without a slow frame.
- A memory mode that adapts to how much the machine has free.
- A view of sleeping tabs and what they save.

## Not planned

- Accounts or a sign-in to use Aven.
- Telemetry, or advertising of any kind.
- A cryptocurrency wallet or token.
