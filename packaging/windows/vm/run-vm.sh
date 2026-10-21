#!/usr/bin/env bash
# Windows 10 test VM on QEMU/KVM, fully unattended.
#   run-vm.sh install <win10.iso>   wipe & install Windows into the VM disk, run setup.ps1 at first logon
#   run-vm.sh start                 boot the installed VM
#   run-vm.sh stop | status | screenshot <out.png> | keys <k...> | type <text> | serve
# State lives in $FLIGHTDECK_VM_DIR (default ~/.cache/flightdeck-vm). The guest reaches the
# host as 10.0.2.2; the guest's port 7799 (Flightdeck MCP) is forwarded to 127.0.0.1:7799 and
# `serve` offers dist/ (MSI + setup.ps1) on http://127.0.0.1:8000 for setup.ps1.
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
repo=$(cd "$here/../../.." && pwd)
vmdir=${FLIGHTDECK_VM_DIR:-$HOME/.cache/flightdeck-vm}
disk=$vmdir/win10.qcow2
qmp=$vmdir/qmp.sock
pidfile=$vmdir/qemu.pid
mkdir -p "$vmdir"

unattend_image() {
  local img=$vmdir/unattend.img
  rm -f "$img"
  mkfs.vfat -C "$img" 1440 >/dev/null
  mcopy -i "$img" "$here/autounattend.xml" ::autounattend.xml
  echo "$img"
}

boot() {
  local extra=("$@")
  qemu-system-x86_64 -name flightdeck-win10 -enable-kvm -cpu host -smp 4 -m 6G -machine pc \
    -drive "file=$disk,if=virtio,format=qcow2" \
    -netdev user,id=n0,hostfwd=tcp:127.0.0.1:7799-:7799 -device e1000,netdev=n0 \
    -vga std -display none -usb -device usb-tablet \
    -rtc base=utc -qmp "unix:$qmp,server,nowait" -pidfile "$pidfile" -daemonize "${extra[@]}"
}

case "${1:-}" in
  install)
    iso=${2:?path to the Windows 10 ISO}
    [ -f "$pidfile" ] && kill "$(cat "$pidfile")" 2>/dev/null || true
    rm -f "$disk"; qemu-img create -f qcow2 "$disk" 64G >/dev/null
    # virtio storage needs a driver Windows lacks; install onto IDE instead and keep it that way.
    img=$(unattend_image)
    qemu-system-x86_64 -name flightdeck-win10 -enable-kvm -cpu host -smp 4 -m 6G -machine pc \
      -drive "file=$disk,if=ide,format=qcow2" -cdrom "$iso" -boot once=d,menu=off \
      -drive "if=floppy,format=raw,file=$img" \
      -netdev user,id=n0,hostfwd=tcp:127.0.0.1:7799-:7799 -device e1000,netdev=n0 \
      -vga std -display none -usb -device usb-tablet \
      -rtc base=utc -qmp "unix:$qmp,server,nowait" -pidfile "$pidfile" -daemonize
    # "Press any key to boot from CD": tap a key for the first seconds.
    for _ in $(seq 1 12); do sleep 1; python3 "$here/qmp.py" "$qmp" keys spc >/dev/null 2>&1 || true; done
    echo "installing; watch with: $0 screenshot /tmp/vm.png  ·  pid $(cat "$pidfile")"
    ;;
  start)
    [ -f "$pidfile" ] && kill "$(cat "$pidfile")" 2>/dev/null || true
    qemu-system-x86_64 -name flightdeck-win10 -enable-kvm -cpu host -smp 4 -m 6G -machine pc \
      -drive "file=$disk,if=ide,format=qcow2" \
      -netdev user,id=n0,hostfwd=tcp:127.0.0.1:7799-:7799 -device e1000,netdev=n0 \
      -vga std -display none -usb -device usb-tablet \
      -rtc base=utc -qmp "unix:$qmp,server,nowait" -pidfile "$pidfile" -daemonize
    echo "started; pid $(cat "$pidfile")"
    ;;
  stop) [ -f "$pidfile" ] && kill "$(cat "$pidfile")" && echo stopped || echo "not running" ;;
  status) python3 "$here/qmp.py" "$qmp" status ;;
  screenshot) python3 "$here/qmp.py" "$qmp" screendump "${2:?out.png}" ;;
  keys) shift; python3 "$here/qmp.py" "$qmp" keys "$@" ;;
  type) shift; python3 "$here/qmp.py" "$qmp" type "$@" ;;
  serve)
    # dist/ plus setup.ps1 and a one-line /msi-name; progress pings from the guest show in the log.
    srv=$vmdir/serve; rm -rf "$srv"; mkdir -p "$srv/vm-stage"
    cp "$repo"/dist/flightdeck-*-x64.msi "$srv/" 2>/dev/null || { echo "no MSI in dist/ (run packaging/windows/build.sh with msitools installed)" >&2; exit 1; }
    cp "$here/setup.ps1" "$srv/"; basename "$(ls "$srv"/*.msi | head -1)" > "$srv/msi-name"
    for s in logon downloaded started exe-missing; do : > "$srv/vm-stage/$s"; done
    cd "$srv" && exec python3 -m http.server 8000 --bind 127.0.0.1
    ;;
  *) sed -n 2,8p "$0"; exit 1 ;;
esac
