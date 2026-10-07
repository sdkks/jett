SHELL := /bin/bash
.DEFAULT_GOAL := help

# Container-first: all cargo verification runs in Podman, never on the host.
# The test suite creates and deletes files under /tmp inside the container.
IMAGE := docker.io/library/rust:latest
PODMAN := podman run --rm --user 1000:1000 -v "$$PWD":/work -w /work -v jett-registry:/usr/local/cargo/registry:U $(IMAGE)

.PHONY: help install-hooks install version bump-version release fmt fmt-check lint test image run clean

help: ## Show developer commands
	@awk 'BEGIN {FS = ":.*## "} /^[a-z-]+:.*## / {printf "  %-14s %s\n", $$1, $$2}' $(MAKEFILE_LIST)

install-hooks: ## Activate tracked .githooks (conventional commits, tracker-name leak guard)
	@bash scripts/install-hooks.sh

install: ## Build and install the release binary into ~/.cargo/bin (never executes it)
	@for tool in git cargo; do command -v "$$tool" >/dev/null || exit 1; done
	cargo install --path . --locked --force
	@printf 'installed jett %s to ~/.cargo/bin (binary not executed; run it inside a container)\n' "$$(sed -n 's/^[ \t]*version[ \t]*=[ \t]*"\(.*\)"/\1/p' Cargo.toml | head -1)"

version: ## Print the crate version from its manifest (host-safe, does not run the binary)
	@printf 'jett %s\n' "$$(sed -n 's/^[ \t]*version[ \t]*=[ \t]*"\(.*\)"/\1/p' Cargo.toml | head -1)"

bump-version: ## Bump version and stamp CHANGELOG (BUMP=major|minor|patch or VERSION=x.y.z)
	@bash scripts/bump-version.sh "$(BUMP)" "$(VERSION)"

release: ## One-shot release: bump, commit, and tag vX.Y.Z (BUMP/VERSION as bump-version; never pushes)
	@bash scripts/release.sh "$(BUMP)" "$(VERSION)"

fmt: ## Format Rust source (inside the container)
	@$(PODMAN) sh -c 'rustup component add rustfmt >/dev/null 2>&1; cargo fmt --all'

fmt-check: ## Check formatting without rewriting (inside the container)
	@$(PODMAN) sh -c 'rustup component add rustfmt >/dev/null 2>&1; cargo fmt --all -- --check'

lint: ## Clippy inside the container; warnings fail
	@$(PODMAN) sh -c 'rustup component add clippy >/dev/null 2>&1; cargo clippy --all-targets --locked -- -D warnings'

test: ## Run the full test suite inside the container
	$(PODMAN) cargo test --locked

image: ## Build the release container image (jett:dev)
	podman build -t jett:dev .

run: ## Run jett in a throwaway container; ARGS='--apparent-size /tmp/<folder-inside-container>'
	podman run --rm -it jett:dev $(ARGS)

clean: ## Remove Cargo build artifacts
	cargo clean
