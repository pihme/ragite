#!/usr/bin/env bash
# Quick check for a release-plz pull request. It replaces the full test suite
# on release-plz-* branches, because such a pull request only bumps versions
# and changelogs: the code it releases was already tested on main.
#
# Fails if the branch changes anything besides the release files, if the two
# crates disagree on the version, if Cargo.lock does not match, or if a
# changelog has no section for the new version.
set -euo pipefail

base="${1:-origin/main}"
allowed='^(Cargo\.lock|crates/ragite-(core|sqlite)/(Cargo\.toml|CHANGELOG\.md))$'

changed=$(git diff --name-only "$(git merge-base "$base" HEAD)" HEAD)
echo "Changed files:"
echo "$changed"
other=$(printf '%s\n' "$changed" | grep -Ev "$allowed" | grep -v '^$' || true)
if [ -n "$other" ]; then
  echo "::error::A release pull request may change only versions and changelogs. Also changed: $(echo $other)"
  exit 1
fi

crate_version() { sed -n 's/^version = "\(.*\)"$/\1/p' "crates/$1/Cargo.toml" | head -n1; }
lock_version() { awk -v n="$1" '$0 == "name = \"" n "\"" { getline; gsub(/version = |"/, ""); print; exit }' Cargo.lock; }

core=$(crate_version ragite-core)
sqlite=$(crate_version ragite-sqlite)
echo "ragite-core $core, ragite-sqlite $sqlite"
if [ -z "$core" ] || [ "$core" != "$sqlite" ]; then
  echo "::error::ragite-core ($core) and ragite-sqlite ($sqlite) must share one version."
  exit 1
fi

for crate in ragite-core ragite-sqlite; do
  locked=$(lock_version "$crate")
  if [ "$locked" != "$core" ]; then
    echo "::error::Cargo.lock has $crate $locked, Cargo.toml has $core."
    exit 1
  fi
done

# ragite-core is tag-only and its changelog may skip a version without changes.
if ! grep -Eq "^## \[$(printf '%s' "$core" | sed 's/\./\\./g')\]" crates/ragite-sqlite/CHANGELOG.md; then
  echo "::error::crates/ragite-sqlite/CHANGELOG.md has no section for $core."
  exit 1
fi

echo "Release pull request for $core is consistent."
