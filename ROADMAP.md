# Roadmap

Where Aven is going. Nothing here is a promise or a date: it is the order
things are being built in, and it changes as people use Aven and say what is
missing. What already ships is in the [README](README.md) and the
[changelog](CHANGELOG.md).

To weigh in, open an issue or a discussion. To build one of these, say so on
its issue first so two people do not do it twice.

## Now

Being built or tested.

- **Setups.** Save how Aven looks and is laid out under a name, switch
  between setups, and carry one to another machine as a file. A setup can
  change appearance and layout and nothing else.
- **Community setups.** A gallery of setups people share, kept as files in
  this repository and read from it, with no server of Aven's own. Sharing one
  is a pull request.
- **An offer to save a password** when you sign in somewhere new, instead of
  saving from the site menu only. Never saved without a yes.

## Next

Designed, with the groundwork in the repository.

- **Sync between your devices**, end-to-end encrypted: settings and
  bookmarks first. No account: one device shows a code, the other types it.
  The relay (in [`relay/`](relay/README.md)) stores only what it cannot read.
- **Working together on a canvas.** A room link lets a few people see and
  edit the same Work canvas live, through the same relay, encrypted between
  them.
- **Ask from the address bar.** Type `>` and a task, and the agent starts on
  the page in front, with the usual consent before it acts.
- **Tidy my tabs.** One command sorts open tabs into groups by what they are
  about, and shows the result before changing anything.
- **Panic key.** One shortcut hides every window and mutes every tab, and
  brings them back as they were.

## Later

Wanted, not started.

**Privacy and security**

- A signed installer, so Windows stops warning about an unknown publisher.
- An encrypted export of a whole profile, to move it or keep a backup.
- A privacy report: what was blocked, for which sites, over time.
- One place to see and revoke what each site is allowed to do.

**Agents**

- Routines: a task an agent repeats on a schedule, with a log of each run.
- Local models through Ollama and other local servers, set up from Settings.
- More for outside agents over MCP: downloads, bookmarks, and a session
  scoped to chosen tabs.

**Play**

- Game mode profiles: different limits for different games.
- A network and frame-time overlay for the game in front.
- More sidebar apps, and controls for the stream or call in them.

**Make it yours**

- Themes with more than colours: shapes, spacing and sounds.
- A marketplace page on the web for setups and themes.
- Custom keyboard shortcuts carried in a setup.

**Everywhere**

- `winget install` and a portable build.
- A beta channel for updates.
- macOS checked and released; Linux looked at seriously.
- A site for Aven, with the documentation.

**Speed and weight**

- A startup budget, measured at each release.
- Less work when switching between settings pages.
- A memory mode that adapts to how much the machine has free.

## Not planned

- Accounts or a sign-in to use Aven.
- Telemetry, or advertising of any kind.
- A cryptocurrency wallet or token.
