#!/usr/bin/env bash
set -euo pipefail

# Read current version from Cargo.toml
current=$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)"/\1/')
IFS='.' read -r major minor patch <<< "$current"

# Bump minor, reset patch
new_minor=$((minor + 1))
new_version="${major}.${new_minor}.0"
tag="v${new_version}"

echo "Bumping version: ${current} → ${new_version}"

# Update Cargo.toml
sed -i '' "s/^version = \"${current}\"/version = \"${new_version}\"/" Cargo.toml

# Update Cargo.lock
cargo check --quiet 2>/dev/null

# Commit, tag, push
git add Cargo.toml Cargo.lock
git commit -m "chore: bump version to ${new_version}"
git tag "${tag}"
git push origin main --tags

echo "Done — pushed ${tag}"
