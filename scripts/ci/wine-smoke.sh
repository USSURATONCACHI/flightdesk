#!/usr/bin/env bash
# Smoke-test the Windows packages under Wine: both MSIs install and uninstall cleanly (each in a
# fresh prefix), and the installed flightdeck.exe starts, opens a BPMN file and answers over MCP.
# Needs Wine >= 9 (the Rust std imports bcryptprimitives.dll), Xvfb (or a DISPLAY) and python3. Usage: wine-smoke.sh [dist-dir]
set -euo pipefail
dist=$(cd "${1:-dist}" && pwd)
here=$(cd "$(dirname "$0")" && pwd)
fixture="$here/../../crates/bpmn/tests/fixtures/order.bpmn"
export WINEDEBUG=-all WINEDLLOVERRIDES="mscoree,mshtml="
# Headless CI has only a software Vulkan device; DirectX 12 through vkd3d loses it during setup,
# so the smoke test renders through Vulkan (winevulkan -> host lavapipe). Users keep the default.
export WGPU_BACKEND="${WGPU_BACKEND:-vulkan}"
embedded=$(ls "$dist"/flightdeck-*-x64.msi)
portable=$(ls -d "$dist"/flightdeck-*-windows-x86_64)

# One X server for the whole run: Wine's desktop process starts with the first Wine command
# (wineboot) and keeps whatever display it saw then, so the display must exist from the start.
if [ -z "${DISPLAY:-}" ]; then
  Xvfb :87 -screen 0 1600x1000x24 -nolisten tcp >/dev/null 2>&1 &
  xvfb=$!
  trap 'kill $xvfb 2>/dev/null || true' EXIT
  export DISPLAY=:87
  sleep 2
fi

check_installer() {
  local label=$1 msi=$2
  export WINEPREFIX; WINEPREFIX=$(mktemp -d)
  wineboot -u >/dev/null 2>&1
  wine reg add 'HKCU\Software\Wine\WineDbg' /v ShowCrashDialog /t REG_DWORD /d 0 /f >/dev/null 2>&1
  local pf="$WINEPREFIX/drive_c/Program Files/Flightdeck"
  wine msiexec /i "$(winepath -w "$msi")" /qn
  [ -f "$pf/flightdeck.exe" ] && [ -f "$pf/locales/ru-RU.ftl" ] || { echo "::error::$label: files missing after install"; exit 1; }
  echo "$label: installed ($(find "$pf" -type f | wc -l) files)"
  if [ "$label" = embedded ]; then
    bash -c '
      wine "$1" --mcp --mcp-port 7701 --no-focus --size 1200x800 --bpmn "$(winepath -w "$2")" >"$WINEPREFIX/app.log" 2>&1 &
      app=$!
      trap "kill $app 2>/dev/null; wineserver -k 2>/dev/null || true" EXIT
      python3 "$3/mcp.py" 7701 wait 120 || { echo "::error::app did not start; its log:"; grep -vE "MESA|pci id|^[0-9a-f]+:fixme" "$WINEPREFIX/app.log" | tail -40; exit 1; }
      python3 "$3/mcp.py" 7701 call ui_state | python3 -c "import json,sys; s=json.load(sys.stdin); d=s[\"place\"][\"diagram\"]; assert d[\"loaded\"] and d[\"elements\"]>10, s; print(\"app: diagram loaded with\", d[\"elements\"], \"elements\")"
      python3 "$3/mcp.py" 7701 call ui_settings | python3 -c "import json,sys; s=json.load(sys.stdin); tags={p[\"tag\"] for p in s[\"languages_available\"]}; assert {\"en-US\",\"ru-RU\",\"de-DE\"}<=tags, tags; print(\"app: language packs\", sorted(tags))"
    ' _ "$pf/flightdeck.exe" "$fixture" "$here"
  fi
  wine msiexec /x "$(winepath -w "$msi")" /qn
  local left; left=$( { find "$pf" -type f 2>/dev/null || true; } | wc -l)
  [ "$left" -eq 0 ] || { echo "::error::$label: $left files left after uninstall"; exit 1; }
  echo "$label: uninstalled cleanly"
  wineserver -k 2>/dev/null || true
  rm -rf "$WINEPREFIX"
}

check_installer embedded "$embedded"
check_installer directory "$portable/flightdeck-setup.msi"
echo "wine smoke test passed"
