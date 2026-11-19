# Notices

Aven is licensed under the Mozilla Public License 2.0 (see `LICENSE`).

## Based on Zephium

Aven is built on the source code of [Zephium](https://github.com/zephium-browser/Zephium),
an open-source browser published under the Mozilla Public License 2.0, taken
at upstream commit `9026a1f`. Its code stays under the MPL 2.0, as the
licence requires.

Aven renames the product and its code (crates, packages, identifiers and
interface text) from Zephium to Aven, replaces the logo and the wordmark,
and turns off what depends on the upstream project's servers:

- accounts and the hosted models they give access to (no account server is
  ever contacted; models work through the person's own keys or a local
  server such as Ollama or LM Studio);
- automatic updates, which would be signed with the upstream project's key.

Aven is a modified version of that code. Beyond the renaming, it changes
existing files and adds its own, among them the vault and its fill, the task
manager, game mode and its limits, the window controls, forced dark mode,
consent refusal and streamer mode. Those changes are Aven's and are published
under the same licence.

The Zephium name and logo belong to its authors; Aven does not use them, and
the Zephium project does not endorse Aven.

## Third-party material

Filter lists bundled under `assets/blocker-seed` keep their own licences
(CC BY-SA 3.0 and GPL-3.0), listed in that folder's `NOTICE`. Vendored crates
under `vendor/` keep the licences stated in each of them.
