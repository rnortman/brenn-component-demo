.PHONY: check repin bundle bundle-dir run-artifacts run invite e2e setup-hooks scrub-tree

# Extra flags for every bazel invocation. This repository's `.bazelrc` carries
# no `ci`/`cd` stanzas — a consumer module has no way to import brenn's — so a
# lane's cache flags are flattened here from the same-named stanzas in brenn's
# `.bazelrc`, which are the canonical text. Every module that builds against
# brenn from its own output base carries a copy; a flag change is made in
# brenn's `.bazelrc` and then transcribed into each. Unset runs with this
# repository's defaults.
LANE ?=
ifeq ($(LANE),)
LANE_FLAGS :=
else ifeq ($(LANE),ci)
LANE_FLAGS := --disk_cache=$(CURDIR)/.bazel-disk-cache --experimental_disk_cache_gc_max_size=8G --repository_cache=$(CURDIR)/.bazel-repo-cache --repo_contents_cache= --show_timestamps --announce_rc --color=no --curses=no --keep_going
else ifeq ($(LANE),cd)
LANE_FLAGS := --disk_cache=$(HOME)/.build-cache/bazel-disk --experimental_disk_cache_gc_max_size=40G --experimental_disk_cache_gc_max_age=30d --repository_cache=$(HOME)/.build-cache/bazel-repo --show_timestamps --announce_rc --color=no --curses=no --keep_going
else
$(error LANE must be ci, cd or unset, not "$(LANE)")
endif

# Work against an unpushed brenn. `MODULE.bazel`'s `git_override` fetches from
# public GitHub, so a pin that is not on `main` fails at module resolution; this
# points the module graph at a local checkout instead. Never set in CI.
BRENN_OVERRIDE ?=
ifneq ($(BRENN_OVERRIDE),)
LANE_FLAGS += --override_module=brenn=$(BRENN_OVERRIDE)
endif

BAZEL_FLAGS := $(LANE_FLAGS)

# The gate. Both guests compiled for wasm32, both artifacts transpiled and
# staged, the backend package emitted, the bundle assembled and contract-checked,
# and both deployer documents compiled against the module roots a host resolves
# them from — plus brenn's clippy and rustfmt aspects over this repository's own
# crates.
# Two invocations, as brenn's own gate is. An aspect only applies to top-level
# targets, and both guest crates are reached through the component rule's
# platform transition, so clippy needs its own request under the wasm32 platform
# to see them at all.
check:
	bazel test $(BAZEL_FLAGS) --config=clippy --config=rustfmt //...
	bazel build $(BAZEL_FLAGS) --config=clippy --config=rustfmt --platforms=@brenn//bazel/platforms:wasm32 //demo-counter:demo-counter //demo-panel:demo-panel

# Re-resolve this repository's third-party crate hub after a `Cargo.toml` or
# `Cargo.lock` change. Writes `cargo-bazel-lock.json`, which is tracked.
repin:
	CARGO_BAZEL_REPIN=true bazel mod $(BAZEL_FLAGS) deps

# The release tree a deploy pipeline tars: `components/`, `surface/`, `modules/`
# and the manifest grammar, contract-checked by `//:bundle_contract`.
bundle:
	bazel build $(BAZEL_FLAGS) //:bundle

# Print the absolute path of that tree. A packaging step needs the path and
# cannot spell it: the configuration segment of an output path is Bazel's to
# choose, and the lane flags that decide which output base was used live in this
# file. Fail-closed on every leg, so a caller never tars an empty string or a
# tree `bundle` did not build.
bundle-dir:
	@set -e; \
	root=$$(bazel info $(BAZEL_FLAGS) execution_root); \
	rel=$$(bazel cquery $(BAZEL_FLAGS) --output=files //:bundle); \
	test -n "$$root" || { echo "bazel info printed no execution_root" >&2; exit 1; }; \
	test -n "$$rel" || { echo "bazel cquery printed no output path for //:bundle" >&2; exit 1; }; \
	test -d "$$root/$$rel" || { echo "$$root/$$rel is not a directory; run make bundle first" >&2; exit 1; }; \
	printf '%s/%s\n' "$$root" "$$rel"


# --- Running the whole system from this checkout -----------------------------

# Bazel's convenience symlinks (`--symlink_prefix=.bazel-` in `.bazelrc`).
# `.bazel-bin` is this module's output tree; the execroot is how source files of
# a dependency module are reached, and brenn's config module root is a source
# tree, not a build output.
#
# The execroot is asked for rather than spelled: its convenience symlink is
# named after the checkout's directory, which is whatever the clone landed in
# and nothing this repository controls. Recursive (`=`), so an unbuilt tree does
# not pay for the query until a target that runs a server needs it.
BAZEL_BIN := .bazel-bin
BRENN_EXEC = $(shell bazel info $(BAZEL_FLAGS) execution_root)/external/brenn+
BRENN_BIN := $(BAZEL_BIN)/external/brenn+

# Six roots: brenn's three, built here rather than installed, and this bundle's
# three. `--modules` is a root-level flag (the compiler needs it for every
# verb); `--components` and `--surface` are `serve`-only.
MODULE_ROOTS := --modules $(BRENN_EXEC)/config/specs \
	--modules $(BAZEL_BIN)/bundle/modules
SERVE_ROOTS := --components $(BRENN_BIN)/brenn-wasm/install_tree \
	--surface $(BRENN_BIN)/surface/dist \
	--components $(BAZEL_BIN)/bundle/components \
	--surface $(BAZEL_BIN)/bundle/surface

BRENN := $(BRENN_BIN)/brenn/brenn

RUN_TARGETS := @brenn//brenn:brenn @brenn//frontend:dist @brenn//surface:dist \
	@brenn//brenn-wasm:install_tree //:bundle //:noop_mcp

run-artifacts:
	bazel build $(BAZEL_FLAGS) $(RUN_TARGETS)

# Pages at /surface/demo and /surface/demo-split. `make invite` mints a
# registration link.
run: run-artifacts
	$(BRENN) --config config/dev.brenn $(MODULE_ROOTS) serve $(SERVE_ROOTS)

invite: run-artifacts
	@$(BRENN) --config config/dev.brenn $(MODULE_ROOTS) invite

E2E_BASE_URL := http://127.0.0.1:3300

e2e/node_modules: e2e/package-lock.json
	cd e2e && npm ci

# Browser-level end-to-end tests (Playwright). Fresh state every run. The
# server is backgrounded with no output redirection — a pipe against a
# backgrounded process hangs — and always stopped by the trap.
e2e: run-artifacts e2e/node_modules
	@cd e2e && node -e "const{chromium}=require('@playwright/test');const fs=require('fs');const p=chromium.executablePath();if(!fs.existsSync(p)){console.error('ERROR: Playwright chromium browser not installed. Run: cd e2e && npx playwright install chromium');process.exit(1);}"
	@rm -rf target/e2e
	@mkdir -p target/e2e
	@set -e; \
	if curl -sf -o /dev/null $(E2E_BASE_URL)/auth/login 2>/dev/null; then \
	    echo "ERROR: $(E2E_BASE_URL) is already serving before we started — a leaked e2e server or a port clash on 3300. Kill it before running make e2e."; exit 1; \
	fi; \
	invite=$$($(BRENN) --config config/e2e.brenn $(MODULE_ROOTS) invite); \
	$(BRENN) --config config/e2e.brenn $(MODULE_ROOTS) serve $(SERVE_ROOTS) & \
	srv=$$!; \
	trap 'kill $$srv 2>/dev/null || true' EXIT INT TERM; \
	echo "e2e: server PID $$srv; polling $(E2E_BASE_URL)/auth/login ..."; \
	ready=0; \
	for i in $$(seq 1 60); do \
	    kill -0 $$srv 2>/dev/null || { echo "ERROR: e2e server exited before becoming ready"; exit 1; }; \
	    if curl -sf -o /dev/null $(E2E_BASE_URL)/auth/login; then ready=1; break; fi; \
	    sleep 1; \
	done; \
	[ "$$ready" -eq 1 ] || { echo "ERROR: e2e server not ready within 60s"; exit 1; }; \
	cd e2e && BRENN_E2E_BASE_URL=$(E2E_BASE_URL) BRENN_E2E_INVITE=$$invite npx playwright test


# --- Repo tooling ------------------------------------------------------------

# Point git at the tracked hooks. Idempotent; once per clone.
#
# `brenn-scrub` is not built here. It is brenn's crate, installed from a brenn
# checkout, and this repository is Bazel-only with no host binaries of its own
# — so both tools are prerequisites rather than products. A missing one is
# reported rather than papered over: a gate that silently no-ops is a false
# green, which is worse than a red one.
setup-hooks:
	git config core.hooksPath .githooks
	@rm -f .git/hooks/pre-commit
	@command -v brenn-scrub >/dev/null 2>&1 || { \
	    echo "brenn-scrub not found on PATH."; \
	    echo "Install it from a brenn checkout: cargo install --path scrub"; \
	}
	@command -v gitleaks >/dev/null 2>&1 || { \
	    echo "gitleaks not found on PATH."; \
	    echo "Install the pinned release from https://github.com/gitleaks/gitleaks/releases"; \
	    echo "(the pinned version is GITLEAKS_VERSION in .github/workflows/ci.yml)"; \
	}
	@echo "setup-hooks: done."

# Scan every tracked file. The local stand-in for CI's scrub job, and the
# command a clean tree is declared on. Tracked and staged content only —
# untracked files are invisible to it.
scrub-tree:
	brenn-scrub tree
