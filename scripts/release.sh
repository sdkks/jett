#!/usr/bin/env bash
# One-shot local release: bump the version (via bump-version.sh), commit the
# three stamped files (Cargo.toml, Cargo.lock, CHANGELOG.md) with a
# conventional message, and create the annotated vX.Y.Z tag. Never pushes:
# review the printed next steps, then push the branch and the tag yourself.
#
# Refuses to run when tracked files other than the stamped trio have pending
# changes, and restores the stamped files if post-bump validation fails, so a
# failed run leaves the tree exactly as it started.
set -euo pipefail

bump=${1:-}
exact=${2:-}

cd "$(git rev-parse --show-toplevel)"

# Fail fast on unrelated tracked changes: the release commit must contain
# exactly Cargo.toml, Cargo.lock, and CHANGELOG.md.
dirty=$(git status --porcelain --untracked-files=no | awk '
  { path = substr($0, 4) }
  path != "Cargo.toml" && path != "Cargo.lock" && path != "CHANGELOG.md" { print path }
')
if [[ -n "$dirty" ]]; then
  printf 'Refusing to cut a release with unrelated tracked changes:\n%s\n\nCommit or stash them first, then rerun.\n' "$dirty" >&2
  exit 1
fi

# Snapshot the stamped files so a post-bump failure restores the tree.
snap=$(mktemp -d)
cleanup() { rm -rf "$snap"; }
trap cleanup EXIT
cp Cargo.toml Cargo.lock CHANGELOG.md "$snap"/

restore() {
  cp "$snap"/Cargo.toml "$snap"/Cargo.lock "$snap"/CHANGELOG.md .
  printf 'Restored Cargo.toml, Cargo.lock, and CHANGELOG.md; nothing was committed or tagged.\n' >&2
}

bash scripts/bump-version.sh "$bump" "$exact"

version=$(sed -n 's/^[ \t]*version[ \t]*=[ \t]*"\([0-9][0-9.]*\)".*/\1/p' Cargo.toml | head -1)
if [[ ! "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  printf 'Could not read a stable x.y.z version from Cargo.toml.\n' >&2
  restore
  exit 1
fi
tag="v$version"

if git rev-parse -q --verify "refs/tags/$tag" >/dev/null; then
  printf 'Tag %s already exists.\n' "$tag" >&2
  restore
  exit 1
fi

branch=$(git branch --show-current)

printf 'Release stamp:\n'
git diff --stat -- Cargo.toml Cargo.lock CHANGELOG.md

git add Cargo.toml Cargo.lock CHANGELOG.md
if ! git commit -m "chore(release): release $tag"; then
  printf 'Commit failed (hook rejection?). ' >&2
  restore
  exit 1
fi

if ! git tag -a "$tag" -m "jett $tag"; then
  printf 'Tag %s was not created; the release commit exists. To start over:\n  git reset --hard HEAD~1\nthen rerun this script after removing any partial tag.\n' "$tag" >&2
  exit 1
fi

printf '\nReleased %s on %s (commit + annotated tag created; nothing pushed).\n\nNext steps:\n  git push origin %s %s   # triggers the release workflow (prebuilt binaries)\n  cargo publish --locked  # after the workflow uploads assets — enables cargo binstall jett\n\nUndo before pushing:\n  git reset --hard HEAD~1 && git tag -d %s\n' \
  "$tag" "${branch:-detached-head}" "${branch:-HEAD}" "$tag" "$tag"
