#!/usr/bin/env python3
"""Minimal QMP client for the test VM: `qmp.py <socket> keys <key...>`, `screendump <png>`,
`status`, `quit`, `type <text>` (US layout), `typealt <text>` (any layout, Windows). Keys use QEMU names (ret, spc, ctrl-alt-delete, a, 1 ...)."""
import json, socket, subprocess, sys, time

def connect(path):
    s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    s.connect(path)
    f = s.makefile("rwb", buffering=0)
    json.loads(f.readline())  # greeting
    def cmd(execute, **args):
        f.write((json.dumps({"execute": execute, "arguments": args}) + "\n").encode())
        while True:
            line = json.loads(f.readline())
            if "return" in line or "error" in line:
                return line
    cmd("qmp_capabilities")
    return cmd

def keys(cmd, names):
    for n in names:
        parts = [{"type": "qcode", "data": p} for p in n.split("-")]
        cmd("send-key", keys=parts)
        time.sleep(0.08)

QCODES = {" ": "spc", ".": "dot", "-": "minus", "/": "slash", ":": "shift-semicolon", "\\": "backslash", "_": "shift-minus", "=": "equal", ",": "comma", ";": "semicolon", "'": "apostrophe", "(": "shift-9", ")": "shift-0", "*": "shift-8", "&": "shift-7", "%": "shift-5", "$": "shift-4", "#": "shift-3", "@": "shift-2", "!": "shift-1", "+": "shift-equal", "\"": "shift-apostrophe", "<": "shift-comma", ">": "shift-dot", "?": "shift-slash", "~": "shift-grave_accent", "|": "shift-backslash", "\n": "ret"}

def type_text(cmd, text):
    for ch in text:
        if ch in QCODES:
            keys(cmd, [QCODES[ch]])
        elif ch.isdigit():
            keys(cmd, [ch])
        elif ch.isupper():
            keys(cmd, ["shift-" + ch.lower()])
        else:
            keys(cmd, [ch])

def type_alt(cmd, text):
    """Layout-independent typing for Windows guests: every character as Alt+0NNN on the
    numpad, sent as explicit key down/up events. ASCII only. NumLock must be on in the guest
    (otherwise numpad 2/4/6/8 act as arrows): `qmp.py <sock> keys num_lock` toggles it."""
    def ev(key, down):
        return {"type": "key", "data": {"down": down, "key": {"type": "qcode", "data": key}}}
    for ch in text:
        if ch == "\n":
            cmd("input-send-event", events=[ev("ret", True)]); cmd("input-send-event", events=[ev("ret", False)])
            continue
        code = "0%03d" % ord(ch)
        cmd("input-send-event", events=[ev("alt", True)])
        time.sleep(0.02)
        for d in code:
            cmd("input-send-event", events=[ev("kp_" + d, True)])
            time.sleep(0.02)
            cmd("input-send-event", events=[ev("kp_" + d, False)])
            time.sleep(0.02)
        cmd("input-send-event", events=[ev("alt", False)])
        time.sleep(0.02)

if __name__ == "__main__":
    sock, action, *rest = sys.argv[1:]
    cmd = connect(sock)
    if action == "keys":
        keys(cmd, rest)
    elif action == "type":
        type_text(cmd, " ".join(rest))
    elif action == "typealt":
        type_alt(cmd, " ".join(rest))
    elif action == "screendump":
        out = rest[0]
        ppm = out + ".ppm"
        print(cmd("screendump", filename=ppm))
        subprocess.run(["magick", ppm, out], check=True)
    elif action == "status":
        print(cmd("query-status"))
    elif action == "quit":
        print(cmd("quit"))
    else:
        print(cmd(action, **(json.loads(rest[0]) if rest else {})))
