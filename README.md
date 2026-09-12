# brenn-component-demo

Two components, built outside brenn's repository with brenn's rules, released
as a bundle a deployment installs beside brenn's own.

- **`DemoPanel`** — a button and a number. Page-hosted: it holds `dom`, so it
  ships only as a surface kind.
- **`DemoCounter`** — a running click total. Placement-agnostic: it keeps its
  whole state on a retained channel, so the same artifact runs on the browser
  surface or on the backend, and ships as both a surface kind and a backend
  package.

Two assemblies wire them: `DemoPage` puts both on one page over page-local
planes, and `DemoSplitPage` puts the panel on the page and the counter on the
backend, over ephemeral channels that cross the wire.

Deliberately pointless as a product and complete as a proof. Every path an
out-of-tree author walks — scaffold from a spec, build a guest, transpile for
the page, package for the backend, bundle all three trees, fit-check a wired
config, test both hostings against brenn's real hosts — is walked once here, so
a broken path breaks this repository's CI rather than an author's afternoon.

```
make setup-hooks                 # once per clone: git hooks, and the scrub gate
make check                       # the whole gate
make e2e                         # the two pages in a real browser
make run                         # a server on config/dev.brenn, for your own eyes
make bundle                      # the release tree; make bundle-dir prints its path
make repin                       # after a Cargo.toml or Cargo.lock change
make check BRENN_OVERRIDE=../brenn   # against an unpushed brenn
```

`make run` starts brenn on `config/dev.brenn` over two mounts — brenn's trees,
built here rather than installed, and this bundle's — declared in a mounts
document that brenn's exported `bazel/wasm/dev_mounts.sh` synthesizes on every
spawn, with the bundle mount named as a deployment names it. It serves the two
pages at `/surface/demo` and `/surface/demo-split` on port 3200.
`make invite` mints the registration link for a server `make run` has already
started, reading the document that spawn wrote rather than writing its own. `make e2e` is the same
arrangement on `config/e2e.brenn` and port 3300, with fresh state, driven by
Playwright; both it and `make check` run as gates in `.github/workflows/ci.yml`.
Chromium is the one thing the Makefile leaves to the machine
(`cd e2e && npx playwright install chromium`).

## What is the consumer's to state

bzlmod reads these from the root module only, so this repository repeats them
and brenn cannot supply them:

- **The brenn pin.** `git_override(module_name = "brenn", commit = ...)`. The
  commit must be *served by* brenn's public remote, because the fetch is
  anonymous: one that exists only in a local checkout fails module resolution,
  and so does one that was pushed and later rebased away — GitHub answers `not
  our ref`. That it is also on `main` is policy, checked by nothing, because
  GitHub serves any commit object it holds by SHA. Work against an unpushed
  brenn with `BRENN_OVERRIDE`, never by editing the pin, and never call this
  repository green on an overridden run.
  Build-time contracts — the macros, `brenn-guest`'s API, the scaffold's
  generated shape, `dsl_cli`'s subcommands — are hard-cut at this pin: a bump is
  when a rename or a changed shape is paid for, and never between bumps.
- **The Rust toolchain.** `rust.toolchain(...)` with brenn's `RUST_VERSION` and
  `extra_target_triples = ["wasm32-unknown-unknown"]`. brenn's crates compile
  under this toolchain, so a version below brenn's floor fails at compile.
- **The `fltk` override**, at brenn's commit. fltk is in no registry, and a
  dependency's `git_override` is ignored.
- **The `aspect_rules_js` override.** Through 3.4.1, `npm_package_store`
  computes its store's relative link targets one character short when the store
  sits in an external repository's sub-package, so brenn's jco dies with
  `ERR_MODULE_NOT_FOUND` on its first import. The pinned commit is the upstream
  fix; the override goes away when it is in a release.
- **The fltk serde flag** in `.bazelrc`, spelled
  `--@fltk//crates/fltk-serde-core:serde=@brenn//brenn-dsl:fltk_serde`. Without
  it `@brenn//brenn-dsl:dsl_cli` fails to compile with mismatched-serde-instance
  errors.

And one naming rule: the crate hub is `demo_crates`, not `crates` or
`wasm_crates`; rules_rust refuses two hubs of one name across modules.

Three pins are copied from brenn by hand and nothing holds them equal
mechanically: `.bazelversion`, `RUST_VERSION` in `MODULE.bazel`, and the fltk
commit. Re-copy all three when the brenn pin moves.

## Layout

- `spec/` — the authored specifications, flat. This is a module root: the same
  shape a bundle's `modules/` has once installed, and what a deployment's
  `use @demo-panel::*;` resolves against. `demo-panel.brenn` also carries the
  two assemblies, because a packaged module may ship the arrangements its
  author offers.
- `demo-counter/`, `demo-panel/` — the guest crates. Each takes its port
  surface from `src/spec.rs`, generated from its specification at build time
  and never tracked, and reaches every capability through that module's
  re-exports, so deleting a word from a spec breaks the guest compile.
- `config/` — the deployer's documents. `dev.brenn` carries the environment
  settings and one stamp; `e2e.brenn` is its hermetic twin and differs in a port
  and three state paths and nothing else. Everything they share —
  `assembly DemoDeployment`, holding both pages and the self-description stamps
  `config-check` requires — is `deployment.brenn`, so the arrangement cannot
  drift apart. The `demo_ui` ceiling is the deliberate exception: each root
  declares its own and passes it in, because a ceiling is one deployment's
  consent, so nothing compares the two and a word wanted in both is two edits.
  The self-description vocabulary arrives as a library module, imported in
  `deployment.brenn` with `use @surface-description::*;`, so this repository
  keeps no copy of it.
  Both documents name build outputs of this checkout where a deployment would
  name installed paths — the frontend tree, and the noop MCP script `//:noop_mcp`
  stages out of brenn — so `make run` is the whole system without an install.
- `fit/`, `tools/config_check.sh` — the gates over the deployer documents that
  are not the documents themselves. Each `fit/*.brenn` is a root that must be
  refused, and `BUILD.bazel` generates one target per file from a glob against a
  map of the refusal each must earn, so a fixture nothing names is a build
  failure rather than a case that never runs. The script runs `brenn
  config-check` over both root documents — the verb an installer runs before it
  stops a service, and the only gate here that lowers.
- `tests/` — the host-side tests. The counter is driven twice, once through
  `brenn_wasm::ProcessorComponent` and once through `brenn_page_harness`, and
  both runs are compared against one constant: that is "one artifact, two
  hostings" as an executable claim rather than a README's. The panel is driven
  against the recording page host alone, because `dom` makes it page-only.
- `e2e/` — the Playwright suite. One spec, run for both slugs, asserting the
  same thing of each: the panel mounts showing `0`, and each of three clicks
  makes it read one more, which is the retained-total read a coalesced burst
  would skip. On `demo` that never leaves the page; on `demo-split` every click crosses
  the bus to the backend counter and the total comes back. Selectors are the
  component's own `data-demo-*` markers, never styling hooks.
- `deployed-components.txt` — the backend packages this repository ships. The
  panel is absent on purpose: it is page-hosted only.
- `.gitleaks.toml`, `.githooks/`, `.claude/` — the scrub gate, copied from the
  byte-checked template that lives in brenn. `make setup-hooks` points git at
  `.githooks/` once per clone; from then on a commit scrubs its staged diff and
  then runs `make check`, and a push scrubs its outgoing range. Fix the template
  and re-copy — never edit a copy in place, and never copy between sibling
  repositories.
- `.github/workflows/ci.yml` — brenn's CI shape, copied by hand: the Bazel gate
  and the browser suite in one job, a tree scan with the scanner pinned by both
  version and sha256 in the other. The repository is public, so it gets the same
  secret scan.
- `LICENSE`, `NOTICE`, `CHANGELOG.md`, `CLAUDE.md` — Apache-2.0, as everywhere
  in this ecosystem, and the agent-facing notes on what is particular to this
  repository.

## Adding this bundle to a deployment

`make bundle` builds the release tree and `make bundle-dir` prints its absolute
path; a deploy pipeline tars that tree with its own installer and `VERSION`
beside it. This repository ships neither — a bundle carries components, not a
way to install them.

Install the built bundle as one mount — its three trees under a single
directory, which is the unit a server takes its roots from — declare that
directory in the deployment's mounts document, and add to the config:

```
use @demo-panel::*;
use @surface-description::*;
new demo: DemoPage(slug = "demo", skin = <the deployment's skin>) {
  grants = [dom, log, page-dom, ports];
}
new surface_commons: SurfaceCommons;
new demo_desc: SurfaceDescription(slug = "demo");
new demo_panel_desc: KindDescription(kind = "demo-panel");
new demo_counter_desc: KindDescription(kind = "demo-counter");
new chrome_kind_desc: KindDescription(kind = "chrome");
```

A `KindDescription` per kind the pages instantiate, which is one more than the
kinds this bundle ships: `DemoPage` places brenn's `Chrome` around the panel, so
`chrome` is described here too or the document is refused naming the two
addresses it derives for it.

The body on the page stamp is its ceiling: an arrangement a bundle author wrote
holds what the deployment stamps it with and no more, so the words are the
deployer's to write and the compiler refuses the difference — naming the words
it would have to add. A deployment stamping several arrangements under one
consent writes a `principal` instead and passes it in, which is what
`config/dev.brenn` and `config/deployment.brenn` do here. The description
stamps need no ceiling: they declare channels and confer nothing.

A deployment missing a `SurfaceDescription` or a `KindDescription` is refused
with every missing channel named at once, by `config-check` and so by the bundle
installer's pre-stop check. A missing `SurfaceCommons` is refused by the
error-lane check instead, naming `brenn:surface-errors` alone: the commons stamp
declares the channel `observability.surface_error_channel` points at, and that
validator runs first, so its message is the whole report. Either way the refusal
is before an installer stops a service, and `//:config_check` runs the same
check over this repository's own documents.
