#!/usr/bin/env bash
# Windows build from Linux: cross-compiles with the GNU toolchain (mingw-w64), stages a portable
# folder, zips it, and when wixl (msitools) is present also builds an MSI.
# Output: dist/flightdeck-<version>-windows-x86_64.zip [+ dist/flightdeck-<version>-x64.msi]
set -euo pipefail
cd "$(dirname "$0")/../.."
version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n1)
target=x86_64-pc-windows-gnu
name="flightdeck-$version-windows-x86_64"
stage="dist/$name"
rustup target list --installed | grep -q "$target" || rustup target add "$target"
cargo build --release --locked -p cockpit --target "$target"
rm -rf "$stage" && mkdir -p "$stage/locales" dist
cp "target/$target/release/flightdeck.exe" "$stage/flightdeck.exe"
x86_64-w64-mingw32-strip "$stage/flightdeck.exe" || true
cp locales/*.ftl "$stage/locales/"
cp LICENSE "$stage/LICENSE.txt"
cp README.md "$stage/README.md"
cp packaging/windows/run-in-place.cmd "$stage/"
# .ico with the standard sizes, from the SVG master
tmp=$(mktemp -d)
for s in 16 24 32 48 64 128 256; do rsvg-convert -w "$s" -h "$s" assets/icon.svg -o "$tmp/$s.png"; done
magick "$tmp"/16.png "$tmp"/24.png "$tmp"/32.png "$tmp"/48.png "$tmp"/64.png "$tmp"/128.png "$tmp"/256.png "$stage/flightdeck.ico"
rm -rf "$tmp"
(cd dist && rm -f "$name.zip" && zip -qr "$name.zip" "$name")
echo "dist/$name.zip"
if command -v wixl >/dev/null; then
  wixl -D "Version=$version" -D "Stage=$stage" -a x64 -o "dist/flightdeck-$version-x64.msi" packaging/windows/flightdeck.wxs
  echo "dist/flightdeck-$version-x64.msi"
else
  echo "wixl not found (pacman -S msitools): MSI skipped" >&2
fi
