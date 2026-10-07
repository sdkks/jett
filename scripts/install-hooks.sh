#!/usr/bin/env bash
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"
current=$(git config --get core.hooksPath || test "$?" = 1)
if [[ -n "$current" && "$current" != .githooks ]]; then
  printf 'Refusing conflicting core.hooksPath: %s\n' "$current" >&2
  exit 1
fi
for hook in pre-commit commit-msg post-commit; do
  test -f ".githooks/$hook"
  git ls-files --error-unmatch -- ".githooks/$hook" >/dev/null
  chmod +x ".githooks/$hook"
done
git config --local core.hooksPath .githooks
printf '%s\n' 'Hooks installed from .githooks (local Git configuration).'
