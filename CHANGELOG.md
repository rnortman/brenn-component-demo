# Changelog

Notable changes to brenn-component-demo. Format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versions follow
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Versions here track this repository's own packaging, not brenn's. The brenn
commit these components build against is the `git_override` pin in
`MODULE.bazel`, and a pin bump is a change worth an entry: it is where a
build-time contract is allowed to move.

## [0.2.0] - 2026-09-12

### Changed

- Brenn pin moved from `3317cdd9` to `9e2e0b35` (brenn `v0.21.0`), and the
  `fltk` override with it, to `72db6ea0`. Five brenn releases are crossed;
  four of their changes reach this repository, and the three below are what
  they cost.
- The panel keeps its state on a retained `io state` port rather than in a
  `thread_local!`. Linear memory now lives for one activation on both hosts, so
  the handle of the element the total is written to rides the port between
  activations; both assemblies bind the port on the panel and on brenn's
  `Chrome`, which grew one of its own.
- The dev server takes its roots from a synthesized mounts document rather than
  from `--components`/`--surface`/`--modules` flags, which a server no longer
  accepts. `make run`, `make invite` and `make e2e` build it through the
  generator brenn exports for consumer repositories.
- Releasing this version is what un-withholds the demo kinds on a 0.21 host: a
  0.21 host withholds a surface kind whose asset record it cannot read, and the
  records built at the old pin are a version behind.

## [0.1.0] - 2026-09-04

### Added

- Two components built outside brenn with brenn's rules: `demo-panel`, a
  page-hosted panel drawing a button and a number, and `demo-counter`, a
  placement-agnostic counter that runs either on the browser surface or on the
  backend.
- Bundle assembly with a contract test, producing the `components/`,
  `surface/` and `modules/` release tree a deployment installs beside brenn's
  own release.
- A host-side test crate exercising the real hosts, and a Playwright browser
  suite covering both demo pages.
- CI over two jobs: the Bazel gate with clippy and rustfmt aspects plus the
  browser suite, and a secrets scan against the tracked ruleset with the
  scanner pinned by version and sha256.
- Stamp ceilings on pages, declared through a `demo_ui` principal each
  deployer document owns. The compiler refuses a page that holds authority the
  principal does not carry, so a bundle author conferring a new word is caught
  at build time rather than at deployment.
- A `config-check` gate (`//:config_check`) that lowers both deployer
  documents the way the bundle installer does before it stops a service. A
  self-description stamp missing from the shared assembly is now a red gate
  rather than a boot panic.
- Fit refusal fixtures under `fit/`, one per way the stamp ceiling can fail to
  hold: narrow principal, narrow body, wide body, and the parameter hop the
  shipped documents use. BUILD generates targets from a glob, so a fixture
  nothing names is a build failure.
- Brenn pin set to `3317cdd9` via `git_override`; the `local_path_override`
  development shim is retired.
- The self-description vocabulary (`SurfaceDescription`, `KindDescription`,
  `SurfaceCommons`) is now imported as a brenn-shipped library module rather
  than carried as a local copy; `config/describe.brenn` is deleted.
