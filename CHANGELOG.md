# Changelog

Notable changes to brenn-component-demo. Format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versions follow
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Versions here track this repository's own packaging, not brenn's. The brenn
commit these components build against is the `git_override` pin in
`MODULE.bazel`, and a pin bump is a change worth an entry: it is where a
build-time contract is allowed to move.

## [Unreleased]

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
