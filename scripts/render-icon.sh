#!/usr/bin/env bash
# Re-render the embedded window icon from assets/icon.svg (needs librsvg's rsvg-convert).
set -euo pipefail
cd "$(dirname "$0")/.."
rsvg-convert -w 256 -h 256 assets/icon.svg -o assets/icon-256.png
echo "wrote assets/icon-256.png"
