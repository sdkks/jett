# AGENTS.md — jett

Guidance for all agents (and humans) working in this repository. jett is a
terminal disk-space navigator and reclaimer ("jettison the junk").

## The container rule (binding, no exceptions)

jett **deletes files**. The binary and its test suite are never built-and-run
against a host filesystem. All cargo verification (build, test, clippy, fmt)
runs inside Podman with the standard invocation:

```bash
podman run --rm --user 1000:1000 -v "$PWD":/work -w /work \
  -v jett-registry:/usr/local/cargo/registry:U \
  docker.io/library/rust:latest <cargo command>
```

- `--user 1000:1000` is mandatory for `cargo test`: the UI prints a
  "(CAUTION: running as root)" banner as root, which would desync snapshots.
- The test suite creates and deletes `/tmp/jett_tests/**` **inside the
  container**. Never point the scanner at a host path; demo folders are
  synthetic dirs created inside the container (see README "Running in a
  container").
- Host-side `cargo install` / `make install` (compilation only) is allowed;
  executing the installed binary is not. `make version` reads the manifest
  instead of running the binary — keep it that way.
- If an agent-harness safety guard blocks a shell command, never evade or
  disguise it: rephrase the operation innocently, split it, or stop and ask.

## Git hooks: always install them

After cloning (or if hooks ever seem missing), run `make install-hooks`
before your first commit. The tracked hooks in `.githooks/` enforce
conventional commit messages (`type(scope)!: description`) and block
internal tracker identifiers in every commit; `pre-commit` also runs a
secret scan when gitleaks is installed. Commits that bypass the gates
(`--no-verify` without maintainer instruction) are not acceptable
contributions.

Gitleaks is **local-only by owner decision**: it runs in the pre-commit
hook so developers and agents cannot accidentally stage/push secrets. Do
not add gitleaks (or any paid secret-scanning service) to CI workflows —
the public CI/cloud version is not free, and local pre-commit enforcement
is the chosen control.

## Internal tracker names never leak (binding)

jett's internal ticket/process identifiers — an uppercase type word plus a
number (e.g. the tracker's epic/story/task/bug style) — are harness-internal
and must never appear on public surfaces: git commit messages, GitHub PR
titles and descriptions, issues, `CHANGELOG.md` entries, README or other
documentation, and code comments. Describe a change by its content ("render
composition profile inside folder tiles"), never by its ticket. The commit
hooks enforce this for commit messages and staged content; apply the same
rule manually everywhere the hooks cannot reach (PRs, issues, external
posts).

## Developer commands

`make help` lists everything. The workhorses: `make test`, `make fmt-check`,
`make lint` (clippy, warnings denied), `make image` / `make run` (the
`jett:dev` Ubuntu 26.04 image), `make install`, `make bump-version`. Keep new
gates container-first by default; a target that runs cargo on the host needs
an explicit, written justification in the Makefile comment.

## Testing and snapshots

- The suite is insta 1.x snapshot testing: 45 tests render the real UI through
  a fake backend and compare against committed `.snap` files.
- Snapshot changes are reviewed, never silently accepted: run with
  `INSTA_FORCE_PASS=1`, inspect every diff, and accept only whitespace/
  padding shifts or changes to the rendered `/tmp/jett_tests/...` path.
  Anything else is a rendering regression — stop and diagnose.
- No `.snap.new` files may be left in the tree.
- The event-sequence tests pin the terminal lifecycle exactly. ratatui's
  `Terminal::clear()` queries cursor position, which conflicts with the
  always-running crossterm input thread in a live PTY — startup/shutdown
  clears intentionally use the backend directly. Do not "simplify" this back.

## Versioning and changelog (binding)

- Every user-facing change lands with its `CHANGELOG.md` `[Unreleased]`
  bullet **in the same commit**. See `.pi/skills/changelog/SKILL.md` for the
  full convention.
- Never hand-edit versions or stamp release headings: `make bump-version
  BUMP=patch|minor|major` (or `VERSION=x.y.z`) updates `Cargo.toml`,
  refreshes `Cargo.lock` in the container, and stamps the changelog. Review
  the diff, commit, tag `vX.Y.Z`. The script never commits or tags.
- Pushing the tag runs the release workflow: it uploads prebuilt
  `jett-<target>.tar.gz` assets (Linux/macOS, x86_64/aarch64) matching
  `[package.metadata.binstall]`, so `cargo binstall jett` works. Publishing
  to crates.io is a separate, local `cargo publish --locked` (registry
  credentials never live in CI).
- Historical release sections are immutable; pre-jett history was
  removed by owner decision — do not resurrect it.

## Dependency policy

Any dependency addition or re-pin requires a research step **before** the
Cargo.toml change: verify currency on crates.io (updated_at, cadence,
maintenance), know the 2–3 alternatives and why the choice wins, check
license/MSRV/feature-minimality, and record the rationale in the commit
message (and the changelog bullet if user-visible). "We didn't look" is not
an answer; pins copied from other projects must be re-verified. Current
stack was verified 2026-10: ratatui 0.30, crossterm 0.29, clap 4.6, anyhow,
jwalk 0.9, nix 0.31, insta 1.49.

## Product invariants (do not weaken)

- Deletion always requires confirmation unless the user explicitly passes
  `--disable-delete-confirmation`. Never remove or soften the prompt.
- Freed-space accounting must remain exact (deletions update `freed:` and the
  treemap atomically with the UI refresh). In `--dry-run`, only filesystem
  removal is skipped; the same exact accounting is hypothetical and labeled
  `would free:`. Confirmation is still required unless explicitly disabled.
- The treemap must remain explorable while scanning (index-while-scanning UX).
- `src/os/windows.rs` is intentionally unmodified and unverifiable in the
  Linux container; do not refactor it opportunistically.

## Documentation surfaces (keep in lockstep)

README.md is for users, CONTRIBUTING.md for contributors, AGENTS.md (this
file) for agents, `.pi/skills/` for agent skills. A material change to the
bump/release flow, the make targets, or the test workflow requires a
same-commit review of the surfaces that describe it — especially the
changelog skill. A stale skill silently degrades every agent consumer.
