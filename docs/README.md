# Aven documentation

Code and tests are the evidence of present behaviour. Where a guide and the
code disagree, the code is right and the guide needs fixing:
`security-model.md` is the reference for what is enforced, `architecture.md`
for how the browser is put together.

## Start here

- [Architecture](architecture.md): which part owns what, from the native host
  to the interface.
- [Security model](security-model.md): what each boundary protects, and how.
- [Contributing](CONTRIBUTING.md): setting up, checking a change, sending
  it.

## By area

- [Interface](frontend.md): how `frame/` is organised and what its rules are.
- [Work](work.md): the agent workspace.
- [Agentic browsing](agentic-browsing.md): how an agent observes and acts on a
  page.
- [Ad blocking](adblock.md) and its
  [cosmetic index](adblock-cosmetic-index.md): the filter lists and their
  maintenance.
- [Using extensions](extensions-user-guide.md): installing from the Chrome Web
  Store and what to expect.

## Security upkeep

- [Security maintenance](security-maintenance.md): the recurring checks on
  the web engine's version floor, advisories and forks.
- [Windows hardening review](windows-hardening-review.md): what was changed
  and checked on Windows, entry by entry.
- [Reporting a security problem](SECURITY.md).
