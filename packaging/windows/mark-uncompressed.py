#!/usr/bin/env python3
"""Finish a wixl-built MSI whose payload lives beside it (the portable archive):

1. mark every File row "noncompressed" (attribute 8192);
2. map the install root to the folder holding the .msi (`Flightdeck:.`) and the language packs
   to its `locales/` subfolder, so the source tree equals the archive layout;
3. clear the "compressed" bit in the summary information's word count. While that bit is set
   Windows Installer (and Wine) ignores the source tree and expects every noncompressed file in
   the root next to the .msi; wixl always sets it and msitools cannot change it, so the value is
   patched in place in the property set (same size, so the compound file stays valid).

Uses msibuild/msiinfo from msitools. Usage: mark-uncompressed.py <file.msi>
"""
import struct, subprocess, sys

msi = sys.argv[1]
q = lambda sql: subprocess.run(["msibuild", msi, "-q", sql], check=True)
q("UPDATE `File` SET `Attributes` = 8704")
q("UPDATE `Directory` SET `DefaultDir` = 'Flightdeck:.' WHERE `Directory` = 'INSTALLDIR'")
q("UPDATE `Directory` SET `DefaultDir` = 'locales:locales' WHERE `Directory` = 'LocalesDir'")

FMTID_SUMMARY = bytes.fromhex("E0859FF2F94F6810AB9108002B27B3D9")
PID_WORDCOUNT, VT_I4, COMPRESSED = 15, 3, 2
data = bytearray(open(msi, "rb").read())
pos = data.find(FMTID_SUMMARY)
if pos < 0:
    sys.exit(f"{msi}: summary information not found")
stream = pos - 28                                   # property set header precedes the FMTID
section = stream + struct.unpack_from("<I", data, pos + 16)[0]
count = struct.unpack_from("<I", data, section + 4)[0]
patched = False
for i in range(count):
    pid, off = struct.unpack_from("<II", data, section + 8 + 8 * i)
    if pid == PID_WORDCOUNT:
        vt, value = struct.unpack_from("<Ii", data, section + off)
        if vt != VT_I4:
            sys.exit(f"{msi}: unexpected word count type {vt}")
        struct.pack_into("<i", data, section + off + 4, value & ~COMPRESSED)
        patched = True
if not patched:
    sys.exit(f"{msi}: word count property missing")
open(msi, "wb").write(data)

info = subprocess.run(["msiinfo", "suminfo", msi], check=True, capture_output=True, text=True).stdout
source = next((l for l in info.splitlines() if l.startswith("Source:")), "")
if "(2)" in source or "(3)" in source:
    sys.exit(f"{msi}: compressed bit still set: {source}")
rows = subprocess.run(["msiinfo", "export", msi, "File"], check=True, capture_output=True, text=True).stdout.splitlines()[3:]
print(f"{msi}: {len([r for r in rows if r.strip()])} files noncompressed, {source.strip()}")
