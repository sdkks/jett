# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/)

## [Unreleased]

## [0.1.3] - 2026-10-08


### Fixed
* Build the static musl release binaries with cargo directly — the cross toolchain misdetects ARM runners as x86_64 hosts, so no musl binaries were published for 0.1.2

## [0.1.2] - 2026-10-08

### Changed
* Bumped version to 0.1.2



## [0.1.1] - 2026-10-08


### Added
* Add static musl builds for Linux (x86_64 and aarch64) so prebuilt binaries run on systems regardless of their glibc version, such as NAS firmware and older distributions
* Add `make release` to bump, commit, and tag a release in one step (pushing stays manual)
* Publish prebuilt binaries for Linux and macOS (x86_64 and aarch64) on every tagged release, installable with `cargo binstall jett` or as `.tar.gz` downloads from the releases page
* Add `--dry-run` cleanup planning with unchanged confirmation and accounting, explicit dry-run prompts, an exit summary, and a sorted, deduplicated absolute-path list created by `mktemp`
* Add `--file-list-delim newline|nul|tab|pipe` for cleanup lists; `nul` safely separates all Unix filenames
* Show an always-visible traffic-light chip for dry-run, real deletion, and real deletion with confirmation disabled
* Add an in-TUI theme selector on `t` with name filtering, live preview, Enter to save, and Esc to restore the previous theme without writing config
* Show ranked child-size bars and a counted remainder inside roomy folder tiles, with compact summaries in smaller tiles and observed-only labels while scanning
* Add a semantic theme palette with 19 built-in color schemes plus the legacy `default` appearance, selectable with `--theme <name>`
* Remember theme choices in versioned TOML config and support custom semantic colors, explicit modes, and flag-over-config precedence
* Add `config set theme.scheme <name>` and `config validate`, with non-fatal config notices in the navigator
* Use actively maintained, MIT/Apache-2.0-licensed `toml` and the existing `serde` for config parsing and rewriting instead of a larger config framework or comment-preserving editor

### Changed
* Relabel the `freed:` counter to `would free:` during dry-run to distinguish planned space from actual reclamation
* Keep the default appearance unchanged when neither `--theme` nor a configured theme is supplied

## [0.1.0] - 2026-10-06

### Added
* jett — a terminal disk space navigator and reclaimer (as in jettison)
* Modernized Rust stack: edition 2024 (minimum Rust 1.88), ratatui 0.30, crossterm 0.29, clap 4, anyhow
* Multi-stage Ubuntu 26.04 Containerfile with a non-root runtime
* Container-first development workflow: builds, tests and lints run in Podman, never on the host

### Changed
* MIT license now credits both the original author and the jett maintainer
