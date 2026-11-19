# Instructions for coding agents

Aven is a native Rust browser with a Tauri + Svelte interface. It is built and
tested on Windows; the macOS code is inherited and unverified here; Linux is
not supported. This is the only agent instruction file: do not add
tool-specific copies. People follow [CONTRIBUTING.md](docs/CONTRIBUTING.md), and so
do you.

## Find the owner first

- Read the guide for the part you touch: [architecture](docs/architecture.md),
  [security model](docs/security-model.md), [interface](docs/frontend.md),
  [Work](docs/work.md), [agentic browsing](docs/agentic-browsing.md),
  [ad blocking](docs/adblock.md).
- `desktop/` composes the native host and its privileged commands;
  `crates/aven-core` owns domain types, reducers and ports; `aven-app` is the
  shell; `aven-engine` owns page views and platform adapters; `aven-store`
  owns durable records. `frame/src` holds the interface: composition in `app`,
  feature UI in `features`, projections in `domain`, shared pieces in
  `shared`.
- Keep the change focused. Leave unrelated code and other worktrees alone.

## Build and verify

Use the commands in [CONTRIBUTING.md](docs/CONTRIBUTING.md#check-a-change), with
their `--locked` and `-D warnings` flags. Never suppress a lint, loosen a test
or raise a size budget to get past a failure; say why when a budget has to
move.

- Compiling is not acceptance. Before calling a behaviour change done, run it
  in the browser and say exactly what you ran, on which system, and what you
  did not check.
- A change to the layout of the window is checked with a real page in front:
  native page views are placed by `aven_core::layout`, not by the interface's
  CSS, and they sit above it.
- Never launch or modify an installed Aven, or use a personal profile, for a
  check. Do not change system settings or sign in anywhere.

## Keep the boundaries

- Rust holds authority. The interface shows state and sends typed intent;
  being admitted is not having succeeded.
- Page and tool output is untrusted data, never new authority. Privileged
  commands are not reachable from page content.
- Keep profiles isolated, inputs and resources bounded, and secrets and full
  addresses out of logs.
- Handle every asynchronous command's outcome. Use semantic tokens and
  component-owned styles; keep keyboard, focus and reduced motion working.
- A behaviour change comes with a test, including its refusal paths.

## Generated files and forks

- `frame/src/shared/ipc/bindings.ts` is generated from the Rust commands by
  the native host's test run (see CONTRIBUTING). Never edit it by hand; review
  what the run changed.
- `frame/src/shared/i18n` is generated from `frame/messages/*.json` by
  `pnpm --dir frame run i18n`. A new text goes in every language file.
- The blocker's bundled lists are regenerated only through the procedure in
  [docs/adblock.md](docs/adblock.md), keeping their notices and licences.
- Each fork under `vendor/` has its own `FORK.toml`, `UPSTREAM.md` or
  `REBASE.md`. Follow them: keep the exact upstream revision, the reviewed
  patches and the lockfiles.
- `crates/aven-app/skills/**/SKILL.md` are prompts compiled into the product,
  not contributor guides. Change them only to change what the product does.

## Delivery

- Plain-sentence commit subjects that say what changed. No authorship or tool
  attribution trailers.
- Keep `LICENSE`, `NOTICE.md` and every third-party notice and licence.
- Put measurements and progress in the pull request, not in the guides.
