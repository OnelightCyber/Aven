# Community setups

Setups people share: a theme, fonts, a layout, a new tab. Aven lists what is
here under Settings → Appearance → Setups and saves the one you pick beside
your own. It is applied only when you ask.

## What a setup is

A small JSON file. Export one from Aven (Settings → Appearance → Setups →
Export…) rather than writing it by hand:

```json
{
  "aven_setup": 1,
  "name": "Midnight focus",
  "settings": {
    "appearance": "dark",
    "ui.theme": "midnight"
  }
}
```

A setup can change how Aven looks and is laid out, and nothing else. It
cannot hold a site, an address, a name, privacy or security settings, search
or AI settings: Aven drops such entries when it reads the file, whoever made
it. The settings it may hold are listed in
[`crates/aven-core/src/setups.rs`](../../crates/aven-core/src/setups.rs).

## Share yours

1. Export your setup from Aven and name the file `your-id.aven-setup.json`.
   The id is lowercase letters, digits and dashes, at most 40 characters.
2. Add it to `index.json` with a name, your name or handle, and one line
   about it (140 characters at most).
3. Open a pull request. The tests check that every listed setup has its file
   and that nothing in it is dropped on reading.

By sharing a setup you agree to publish it under the repository's licence.
