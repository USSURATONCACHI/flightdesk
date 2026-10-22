#!/usr/bin/env bash
# Development helper: run the app for a screenshot without stealing focus, placed on the
# top-right corner of the second monitor (X11 path through Xwayland so positioning works).
# usage: scripts/shot.sh out.png [extra args…]
set -euo pipefail
OUT=$1; shift
W=${SHOT_W:-1400}; H=${SHOT_H:-860}
# second monitor geometry from xrandr (falls back to the primary)
GEOM=$(xrandr --query 2>/dev/null | awk '/ connected/{print $0}' | grep -v primary | grep -oE '[0-9]+x[0-9]+\+[0-9]+\+[0-9]+' | head -1 || true)
if [ -z "$GEOM" ]; then GEOM=$(xrandr --query 2>/dev/null | grep -oE '[0-9]+x[0-9]+\+[0-9]+\+[0-9]+' | head -1 || echo "1920x1080+0+0"); fi
MW=${GEOM%%x*}; REST=${GEOM#*x}; MH=${REST%%+*}; REST=${REST#*+}; MX=${REST%%+*}; MY=${REST#*+}
X=$((MX + MW - W)); Y=$MY
BIN=${FLIGHTDECK_BIN:-./target/debug/flightdeck}
# Never touch the developer's real configuration: test runs get their own config/data dirs.
TESTROOT=${FLIGHTDECK_TEST_ROOT:-${TMPDIR:-/tmp}/flightdeck-dev}
export FLIGHTDECK_CONFIG_DIR=${FLIGHTDECK_CONFIG_DIR:-$TESTROOT/config}
export FLIGHTDECK_DATA_DIR=${FLIGHTDECK_DATA_DIR:-$TESTROOT/data}
mkdir -p "$FLIGHTDECK_CONFIG_DIR" "$FLIGHTDECK_DATA_DIR" && chmod 700 "$FLIGHTDECK_CONFIG_DIR" "$FLIGHTDECK_DATA_DIR"
env -u WAYLAND_DISPLAY FLIGHTDECK_NO_FOCUS=1 FLIGHTDECK_ON_TOP=1 FLIGHTDECK_WINDOW_POS="$X,$Y" timeout "${SHOT_TIMEOUT:-120}" "$BIN" --size "${W}x${H}" --screenshot "$OUT" --screenshot-after "${SHOT_AFTER:-150}" --exit-after-screenshot "$@" 2>&1 | grep -E "ERROR|WARN|panick|frames:" || true
