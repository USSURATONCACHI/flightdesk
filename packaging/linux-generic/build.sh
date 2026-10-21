#!/usr/bin/env bash
# Generic Linux archive: binary + language packs + desktop entry + icons + install/run-in-place.
# Output: dist/flightdeck-<version>-linux-<arch>.tar.gz
set -euo pipefail
cd "$(dirname "$0")/../.."
version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n1)
arch=$(uname -m)
name="flightdeck-$version-linux-$arch"
stage="dist/$name"
cargo build --release --locked -p cockpit
rm -rf "$stage" && mkdir -p "$stage/locales" "$stage/icons" dist
install -m755 target/release/flightdeck "$stage/flightdeck"
install -m644 locales/*.ftl "$stage/locales/"
install -m644 packaging/flightdeck.desktop LICENSE README.md "$stage/"
install -m644 assets/icon.svg "$stage/icons/flightdeck.svg"
for s in 16 22 24 32 48 64 128 256 512; do
  rsvg-convert -w "$s" -h "$s" assets/icon.svg -o "$stage/icons/flightdeck-$s.png"
done
install -m755 packaging/linux-generic/install packaging/linux-generic/run-in-place "$stage/"
tar -C dist -czf "dist/$name.tar.gz" "$name"
echo "dist/$name.tar.gz"
