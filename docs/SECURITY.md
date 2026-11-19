# Security

## Reporting a problem

Please report a security problem **privately**, never in a public issue:

- use the **Security › Report a vulnerability** tab of the repository on
  GitHub;
- describe the problem, the versions involved (Aven and WebView2, both shown
  in Settings › About) and, if you can, the steps that show it.

You will get an answer as soon as possible. Once a fix is published, the
problem is described in the [changelog](../CHANGELOG.md), with your name if you
want it there.

## What counts

- a web page that reaches Aven's interface, its internal messages or its
  files;
- a way around the interface's Content Security Policy;
- browsing data (history, bookmarks, private tabs, settings) leaking to a
  site, or to a file where it does not belong;
- the vault's contents, or a saved sign-in, reaching a site it was not saved
  for;
- something sensitive (an address, a title, typed text, a password) written
  to a log;
- a local endpoint of Aven (the Claude Code bridge, the MCP server) answering
  a caller that does not hold its secret, or a web page;
- a shortcut or a dialog of Aven that a page can trigger or imitate.

Problems in the Chromium engine itself go to Microsoft or the Chromium
project. Aven receives their fixes through the WebView2 updates Windows
installs.

## How Aven is protected

The reference is the [security model](security-model.md): what each
boundary protects and how it is enforced. In short, the interface runs in a
locked-down web view of its own, apart from the pages; Rust decides and the
interface only shows state and sends typed requests; and what a page says is
treated as data, never as an instruction.
