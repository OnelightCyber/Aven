# Contributing to Aven

Thanks for wanting to help. This page covers what you need to build Aven,
check a change, and send it.

## Before you start

- Aven is built and tested on **Windows 10 and 11**. The macOS code is
  inherited and has not been built or tested by this project; a change that
  touches it needs someone who can run it there.
- A security problem never goes in an issue or a pull request: see
  [SECURITY.md](SECURITY.md).
- For anything larger than a fix, open an issue first so the direction is
  agreed before you write it.

## Set up

You need [Rust](https://rustup.rs) (the version in `rust-toolchain.toml`
installs itself), Node.js (the version in `.node-version`), pnpm through
Corepack (`corepack enable`), and the
[Tauri prerequisites](https://v2.tauri.app/start/prerequisites/): Microsoft
C++ Build Tools and WebView2 on Windows.

```sh
pnpm install --frozen-lockfile
pnpm dev      # run a development build, with a profile of its own
pnpm build    # package the browser
```

## Where things live

| Folder | What it holds |
|---|---|
| `desktop/` | The native host: windows, privileged commands, the vault, game mode. |
| `crates/aven-core` | Domain types, reducers and the ports the rest implements. |
| `crates/aven-app` | The shell: tabs, windows, layout, preferences. |
| `crates/aven-engine` | Page views and the platform adapters. |
| `frame/` | The interface, in Svelte. |
| `docs/` | The guides: [architecture](architecture.md), [security model](security-model.md), [interface](frontend.md). |
| `vendor/` | Reviewed forks of dependencies, each with its own notes. |

Read the guide for the part you touch before changing it.

## Check a change

Run these from the root. They are what CI runs.

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --locked -p aven-core --lib
cargo test --locked -p aven-app --lib
pnpm --dir frame run check
pnpm --dir frame run build
pnpm --dir frame run check:styles
```

The native host's tests run from its own folder:

```sh
cd desktop
cargo test --features tauri/custom-protocol --lib
```

That run also regenerates the interface's bindings
(`frame/src/shared/ipc/bindings.ts`) from the Rust commands. Never edit that
file by hand: change the Rust side and commit what the run produces. Message
texts live in `frame/messages/*.json`; `pnpm --dir frame run i18n` regenerates
the code that reads them.

A few interface tests are bound to a five-second limit and can fail on a busy
machine. Run the file alone before deciding it is a real failure.

## What a good change looks like

- **It does one thing.** Leave unrelated code as it is.
- **It comes with a test** when it changes behaviour, including the refusal
  paths: the wrong site, the malformed input, the missing permission.
- **It was tried in the running browser**, not only compiled. Say in the pull
  request what you ran, on which system, and what you could not check.
- **It keeps the boundaries.** Rust decides; the interface shows state and
  sends intent. Page content is untrusted data and never gains authority.
  Nothing secret and no full address goes into a log.
- **It never silences a check** to pass: no suppressed lint, no loosened
  test, no raised size budget without saying why.

## Commits and pull requests

Write the commit subject as a plain sentence that says what changed. In the
pull request, give the problem, the change, how you checked it, and what
remains unchecked.

By contributing you agree that your work is published under the
[MPL-2.0](../LICENSE), like the rest of Aven.
