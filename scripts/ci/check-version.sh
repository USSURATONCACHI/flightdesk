#!/usr/bin/env bash
# Fail unless the pushed tag (vX.Y.Z) equals the workspace version in Cargo.toml.
set -euo pipefail
tag=${1:?tag name, e.g. v0.3.0}
version=$(sed -n 's/^version = "\(.*\)"/\1/p' "$(dirname "$0")/../../Cargo.toml" | head -n1)
if [ "$tag" != "v$version" ]; then
  echo "::error::tag $tag does not match Cargo.toml version $version (expected v$version)"
  exit 1
fi
echo "tag $tag matches Cargo.toml version $version"
