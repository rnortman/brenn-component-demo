# Brenn Component Demo

Two components built outside brenn with brenn's rules, packaged as a bundle a
deployment installs beside brenn's own release. Deliberately pointless as a
product and complete as a proof: every path an out-of-tree author walks is
walked once here, so a broken path breaks this repository's CI rather than an
author's afternoon.

Read that as a standing instruction. When something here is awkward, the
interesting question is usually whether brenn's consumer-facing surface is
wrong, not how to work around it locally.

`README.md` covers the components and the layout. This file covers what bites.

## No cargo lane

`Cargo.toml` and `Cargo.lock` exist so `crate_universe` can resolve the one
third-party hub this repository owns. **Cargo never builds this repository** —
`cargo build`/`test`/`clippy` are not expected to work, and a green cargo run
would say nothing about the artifacts that ship. After touching a manifest or
the lockfile run `make repin`; a stale `cargo-bazel-lock.json` fails the build
with a digest mismatch rather than a useful message.

## The brenn pin

`MODULE.bazel` pins the brenn commit every rule, macro and host crate comes
from. Build-time contracts are hard-cut there: a macro rename or a changed
generated shape is paid for when the pin moves, never between bumps.

The pin must be **served by brenn's public remote**, because the fetch is
anonymous. A commit living only in a local checkout fails resolution, and so
does one pushed and later rebased away — GitHub answers `not our ref`. That it
is also on `main` is policy nothing checks: GitHub serves any commit object it
holds by SHA, so an off-`main` pin resolves and then rots quietly.

Work against an unpushed brenn with `make check BRENN_OVERRIDE=<checkout>`,
never by editing the pin. Never set it in CI, and never call this repository
green on an overridden run — an override makes the pin irrelevant, which is
exactly how a pin nothing could resolve went unnoticed once.

`MODULE.bazel` also restates four facts bzlmod reads from the root module only,
so brenn cannot supply them: the Rust toolchain, the `fltk` override, the
`aspect_rules_js` override, and a crate hub named `demo_crates` rather than the
`crates`/`wasm_crates` brenn owns. That duplication is load-bearing, not mess.
`.bazelversion`, `RUST_VERSION` and the fltk commit are copied from brenn by
hand with nothing holding them equal; re-copy all three when the pin moves.

## Gates

`make check` is the gate; `make e2e` drives both pages in a real browser and
needs Playwright's chromium present locally. Both run in CI.

**`git commit` runs `make check` as a pre-commit hook, so commits take minutes.
Give the command a long timeout.**

`LANE=ci` and `LANE=cd` carry cache flags flattened by hand from brenn's
same-named `.bazelrc` stanzas, because a consumer module cannot import a
dependency's `.bazelrc` config. brenn's file is canonical; a change there has
to be copied here.

## Scrub

`brenn-scrub` is brenn's crate and is not built here: install it from a brenn
checkout with `cargo install --path scrub`, plus the pinned gitleaks release.
Run `make setup-hooks` once per clone to point git at `.githooks/`.
`make scrub-tree` sees tracked and staged content only, so an untracked file is
invisible to it.

`.gitleaks.toml`, both `.claude/hooks/` scripts and both `.githooks/` hooks are
copies of a byte-checked template in brenn. Fix the template and re-copy; never
edit a copy in place, and never copy from a sibling repository — every copy had
drifted the one time that was tried.

The tracked `.claude/settings.json` carries the hooks stanza only; per-machine
preferences belong in the gitignored `.claude/settings.local.json`.

## Conventions

A `TODO.md` entry under a slug plus a `TODO(slug)` comment where the work
happens; the slugs are the join key and completing one removes both. Comments
follow brenn's `docs/comment-standard.md`, two of whose rules have a mechanical
scrub gate behind them. The WASM/WIT contract against brenn is an external
contract, so a compatibility shim there is called out at design time or not
taken at all.
