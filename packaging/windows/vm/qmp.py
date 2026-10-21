#!/usr/bin/env python3
"""Minimal QMP client for the test VM: `qmp.py <socket> keys <key...>`, `screendump <png>`,
`status`, `quit`, `type <text>`. Keys use QEMU names (ret, spc, ctrl-alt-delete, a, 1 ...)."""
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

if __name__ == "__main__":
    sock, action, *rest = sys.argv[1:]
    cmd = connect(sock)
    if action == "keys":
        keys(cmd, rest)
    elif action == "type":
        type_text(cmd, " ".join(rest))
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
