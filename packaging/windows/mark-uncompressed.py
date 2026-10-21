#!/usr/bin/env python3
"""Post-process a wixl-built MSI whose payload lives beside it: set the "noncompressed" bit
(8192) on every File row so msiexec reads the files from the directory next to the .msi
instead of looking for a cabinet. Uses msibuild from msitools.
Usage: mark-uncompressed.py <file.msi>"""
import subprocess, sys

msi = sys.argv[1]
subprocess.run(["msibuild", msi, "-q", "UPDATE `File` SET `Attributes` = 8704"], check=True)
rows = subprocess.run(["msiinfo", "export", msi, "File"], check=True, capture_output=True, text=True).stdout.splitlines()[3:]
bad = [r for r in rows if r.strip() and "\t8704\t" not in r]
if bad:
    sys.exit(f"{msi}: attributes not applied: {bad[:2]}")
print(f"{msi}: {len([r for r in rows if r.strip()])} files marked noncompressed")
