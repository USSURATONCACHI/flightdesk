#!/usr/bin/env bash
# Windows 10 test VM on QEMU/KVM, fully unattended.
#   run-vm.sh install <win10.iso>   wipe & install Windows into the VM disk, run setup.ps1 at first logon
#   run-vm.sh start                 boot the installed VM
#   run-vm.sh stop | status | screenshot <out.png> | keys <k...> | type <text> | serve
# State lives in $FLIGHTDECK_VM_DIR (default ~/.cache/flightdeck-vm; put it on a disk that is not
# your system volume). FLIGHTDECK_VM_BPS caps guest disk bandwidth (default 120 MB/s). The guest reaches the
# host as 10.0.2.2; the guest's port 7799 (Flightdeck MCP) is forwarded to 127.0.0.1:7799 and
# `serve` offers dist/ (MSI + setup.ps1) on http://127.0.0.1:8000 for setup.ps1.
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
repo=$(cd "$here/../../.." && pwd)
vmdir=${FLIGHTDECK_VM_DIR:-$HOME/.cache/flightdeck-vm}
# Raw image, direct I/O, bandwidth cap and idle I/O priority: a Windows install otherwise
# saturates the host volume (copy-on-write + page cache) and freezes the desktop.
disk=$vmdir/windows.raw
bps=${FLIGHTDECK_VM_BPS:-120000000}
run_qemu() { ionice -c3 nice -n 10 qemu-system-x86_64 "$@"; }
qmp=$vmdir/qmp.sock
pidfile=$vmdir/qemu.pid
cpu=${FLIGHTDECK_VM_CPU:-host}
unattend=${FLIGHTDECK_VM_UNATTEND:-$here/autounattend.xml}
mkdir -p "$vmdir"

unattend_image() {
  # A second CD with the answer file. (A floppy makes Windows Setup fail with 0x80070570.)
  local img=$vmdir/unattend.iso
  local py=$vmdir/venv/bin/python3
  [ -x "$py" ] || { python3 -m venv "$vmdir/venv" >/dev/null && "$vmdir/venv/bin/pip" install -q pycdlib; }
  "$py" "$here/make-unattend-iso.py" "$unattend" "$img" >/dev/null
  echo "$img"
}

case "${1:-}" in
  install)
    iso=${2:?path to the Windows 10 ISO}
    [ -f "$pidfile" ] && kill "$(cat "$pidfile")" 2>/dev/null || true
    rm -f "$disk"; chattr +C "$vmdir" 2>/dev/null || true; qemu-img create -f raw "$disk" 64G >/dev/null
    # virtio storage needs a driver Windows lacks; install onto IDE instead and keep it that way.
    img=$(unattend_image)
    # q35: AHCI disk/DVD and no floppy controller (a floppy drive, even empty, made Windows
    # Setup fail with 0x80070570 while applying the image).
    run_qemu -name flightdeck-win10 -enable-kvm -cpu "$cpu" -smp 4 -m 6G -machine q35 \
      -drive "if=none,id=hd,file=$disk,format=raw,cache=none,aio=threads,throttling.bps-total=$bps" -device ide-hd,drive=hd,bus=ide.0 \
      -drive "if=none,id=dvd,file=$iso,format=raw,media=cdrom,readonly=on,cache=none" -device ide-cd,drive=dvd,bus=ide.1 \
      -drive "if=none,id=unattend,file=$img,format=raw,media=cdrom,readonly=on" -device ide-cd,drive=unattend,bus=ide.2 \
      -boot once=d,menu=off \
      -netdev user,id=n0,hostfwd=tcp:127.0.0.1:7799-:7799 -device e1000,netdev=n0 \
      -vga std -display none -usb -device usb-tablet \
      -rtc base=utc -qmp "unix:$qmp,server,nowait" -pidfile "$pidfile" -daemonize
    # "Press any key to boot from CD": tap a key for the first seconds.
    for _ in $(seq 1 12); do sleep 1; python3 "$here/qmp.py" "$qmp" keys spc >/dev/null 2>&1 || true; done
    echo "installing; watch with: $0 screenshot /tmp/vm.png  ·  pid $(cat "$pidfile")"
    ;;
  start)
    [ -f "$pidfile" ] && kill "$(cat "$pidfile")" 2>/dev/null || true
    run_qemu -name flightdeck-win10 -enable-kvm -cpu "$cpu" -smp 4 -m 6G -machine q35 \
      -drive "if=none,id=hd,file=$disk,format=raw,cache=none,aio=threads,throttling.bps-total=$bps" -device ide-hd,drive=hd,bus=ide.0 \
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
