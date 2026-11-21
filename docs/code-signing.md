# Code signing policy

Free code signing provided by [SignPath.io](https://about.signpath.io/),
certificate by [SignPath Foundation](https://signpath.org/).

Aven has applied to SignPath Foundation. Until it is accepted, releases carry
no Windows code signature; each is still signed for Aven's own updater.

## What is signed

Two files of each Windows release, and nothing else:

- `aven-desktop.exe`, the browser itself, before it goes into the installer;
- `Aven-Windows-x64-setup.exe`, the installer published on the
  [releases page](https://github.com/OnelightCyber/Aven/releases).

Both are built from this repository by the
[release workflow](../.github/workflows/release.yml), on GitHub-hosted runners,
from a version tag. SignPath checks that each file it signs comes from that
workflow and carries the product name `Aven` and the release's version. The
signing configuration SignPath applies is kept in
[`.signpath/artifact-configurations`](../.signpath/artifact-configurations).

The installer carries the NSIS plug-ins Tauri's installer uses, unchanged
and unsigned by this project: they are not Aven's code and this certificate
does not sign them. When WebView2 is missing, the installer downloads
Microsoft's own installer for it, signed by Microsoft.

## Team roles

| Role | Who |
|---|---|
| Committers and reviewers | [Members of OnelightCyber/Aven with write access](https://github.com/OnelightCyber/Aven/graphs/contributors) |
| Approvers | [OnelightCyber](https://github.com/OnelightCyber), the repository's owner |

Every change from someone who is not a committer goes through a pull request
reviewed by a committer. Every signing request is approved by hand by an
approver before SignPath signs anything. Everyone in these roles uses
multi-factor authentication on GitHub and on SignPath.

## Privacy

What Aven sends, where, and how to turn each one off: [PRIVACY.md](../PRIVACY.md).

## The updater's own signature

Aven also signs each installer with its own update key (minisign), so that an
installed Aven only accepts updates published by this project. That signature
is checked by Aven itself and is separate from the Windows code signature
above.
