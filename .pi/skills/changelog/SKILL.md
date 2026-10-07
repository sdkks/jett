---
name: changelog
description: Keep CHANGELOG.md current under [Unreleased] as user-facing work lands in jett, and cut releases with make bump-version. Use when committing features, fixes, dependency or behavior changes, when asked to update the changelog, or when preparing a release.
---

# Changelog discipline for jett

`CHANGELOG.md` follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
It has one `## [Unreleased]` section at the top and one immutable section per
release. The rule: **a user-facing change and its changelog bullet land in the
same commit.** Do not batch notes for later; later never comes.

## Writing entries

- Add bullets under `## [Unreleased]` using the standard subsections:
  `### Added`, `### Changed`, `### Fixed`, `### Removed`. Create the
  subsection if it does not exist yet; keep subsections in that order.
- One bullet per user-visible change, present tense, backticked identifiers:
  `* Add --apparent-size flag to show real file sizes instead of block usage`.
- Belongs in the changelog: behavior changes, new/removed CLI flags, UI
  rendering changes, platform support, dependency switches users can notice,
  packaging (Containerfile, install), and security fixes.
- Does not belong: pure internal refactors with no observable change, test or
  snapshot churn, formatting, typo-level doc fixes. When unsure, one short
  line under `### Changed` beats silence.

## Cutting a release

Never hand-edit version numbers or stamp release headings yourself:

```bash
make bump-version BUMP=patch   # bump only, or
make release BUMP=patch        # one-shot: bump + commit + annotated vX.Y.Z tag (never pushes)
```

The script bumps `Cargo.toml`, refreshes `Cargo.lock` inside the standard
Podman container, renames `[Unreleased]` to `[x.y.z] - <today>`, inserts a
fresh `[Unreleased]` above it, and fills an empty section with a default
"Bumped version" bullet. `make release` runs that same bump, then commits
the three stamped files as `chore(release): release vX.Y.Z` and creates the
annotated tag — it refuses to run over unrelated tracked changes, restores
the stamped files if anything fails before the commit, and never pushes.
With plain `make bump-version`, review the diff, commit, then tag with
`git tag vX.Y.Z`. Never rewrite or reorder historical release sections.

Two steps complete the release after tagging:

1. `git push origin vX.Y.Z` triggers `.github/workflows/release.yml`, which
   creates the GitHub Release from `CHANGELOG.md` and uploads prebuilt
   `jett-<target>.tar.gz` binaries (Linux and macOS, x86_64/aarch64). Asset
   names and layout must keep matching `[package.metadata.binstall]` in
   `Cargo.toml` — that is what makes `cargo binstall jett` work.
2. Once the workflow's assets are uploaded, publish the crate from the
   maintainer's machine with `cargo publish --locked` from the tagged
   commit. Registry credentials intentionally never live in CI, and
   `cargo binstall` resolves through crates.io, so the release is only
   source-installable until this step lands.
