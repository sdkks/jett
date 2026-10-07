# How to contribute to jett
You can contribute to `jett` in several ways, all of which would be very much appreciated.
Following are a few guidelines that might help you find your way around.

Please note that jett has a Code of Conduct (you can find it in the root of the repository).
It is there so that we can have a safe and pleasant environment that encourages acceptance, participation and kindness.

## Have you found a bug?
If jett is not working for you, or you found something that is not behaving as it should, it would be great if you could let us know about it.
To do this, please open an issue in the repository and provide as much relevant information as possible. Ideally, also a way to reproduce this bug.
If you would like to try and fix this bug yourself, please open a pull request (for more information, see the section of this document regarding code contributions).

## Do you have an idea for a new feature?
`jett` can always be improved, and one of the best ways to improve it is by having its users suggest new features and different ways it can behave.
If you'd like to make such a suggestion, please open an issue detailing it. In this issue others would be able to comment on the suggestion.
Finally, you or someone else would be able to implement this suggestion if it is decided to do so.

## Would you like to make a code contribution?
Code contributions to `jett` are very welcome and encouraged. If you're unsure what to work on, a good place to start is the "Help Wanted" or "Good First Issue" tags in the issues of this repository.

Following is some information you might find useful regarding the particularities of contributing to `jett`:

### Testing
jett uses automated integration tests to make sure everything is working as it should.
If you're adding new functionality, you might want to add new tests or adjust the existing ones.

Always run tests inside a Podman container, not on the host: the tests create and delete directories under `/tmp/jett_tests`. From the repository root, run:

```sh
podman run --rm --user 1000:1000 \
  -v "$PWD":/work -w /work \
  -v jett-registry:/usr/local/cargo/registry:U \
  docker.io/library/rust:latest sh -c 'cargo test'
```

The non-root user keeps snapshots free of the root warning banner. If an earlier root build left `target/` unwritable, add `-e CARGO_TARGET_DIR=/tmp/target` to the `podman run` options.

These tests work by creating textual "snapshots" of how the UI looks in certain situations. One test can have several snapshots.

An example test would be:
1. Create a temporary folder with a few files and subfolders.
2. Run jett on that folder and take a snapshot of its UI. Make sure that snapshot is identical to the snapshot stored in this repository for that test.
3. Enter one of the subfolders, take another snapshot and make sure it is identical to the second stored snapshot for that test.

To store and compare the snapshots, we use [`insta` 1.x](https://insta.rs/docs/).

Optionally install `cargo-insta` inside the development container to review new snapshots, approving or rejecting them. Please see the insta documentation for more details. Review every snapshot change before accepting it, and do not leave `.snap.new` files in the repository.

### Code formatting
`jett` uses [rustfmt](https://github.com/rust-lang/rustfmt) for code formatting. The `make fmt` and `make fmt-check` targets run it inside the same container (`make lint` runs clippy with warnings denied). The component installation is handled for you because the image uses a minimal Rust toolchain.

## Versioning and releases

- `make install-hooks` activates the tracked git hooks before your first
  commit: conventional commit messages (`type(scope)!: description`), a
  guard against internal tracker identifiers leaking into commits, and a
  local gitleaks secret scan (local-only by design; never added to CI).
- `make install` builds and installs the release binary into `~/.cargo/bin` via `cargo install`. It compiles on the host but never executes the binary there — run `jett` inside a container (see README).
- `make version` prints the crate version from its manifest without running the binary.
- `make bump-version BUMP=patch|minor|major` (or `VERSION=x.y.z` for an exact version) updates the `[package]` version, refreshes `Cargo.lock` inside the standard container, and stamps `CHANGELOG.md`: the `[Unreleased]` section becomes `[x.y.z] - <today>`, a fresh `[Unreleased]` is inserted above it, and an empty release section receives a default "Bumped version" note so every bump is recorded.
- The script never commits or tags. Inspect the diff of `Cargo.toml`, `Cargo.lock` and `CHANGELOG.md`, commit, and cut a release with `git tag vX.Y.Z`.
- Pushing the tag (`git push origin vX.Y.Z`) triggers the release workflow (`.github/workflows/release.yml`): it creates the GitHub Release from `CHANGELOG.md` and uploads prebuilt `jett-<target>.tar.gz` binaries (Linux and macOS, x86_64/aarch64) named exactly as `[package.metadata.binstall]` in `Cargo.toml` expects, so `cargo binstall jett` works. The workflow uses only the built-in `github.token` — no secrets are configured.
- Publishing the crate to crates.io is a deliberate local step, so registry credentials never live in CI: once the release workflow has finished uploading assets, run `cargo publish --locked` from the tagged commit. `cargo binstall jett` resolves through crates.io, so until this step the release is only usable via direct downloads or `cargo install`.
- If the lock refresh fails, the script restores `Cargo.toml` and exits; nothing is left half-bumped. Fix the cause and rerun with `VERSION=x.y.z` to retry the same target version.
