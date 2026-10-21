#!/usr/bin/env python3
"""Write autounattend.xml onto a tiny ISO 9660/Joliet image (Windows Setup scans CD roots for
it). Needs the pycdlib package. Usage: make-unattend-iso.py <autounattend.xml> <out.iso>"""
import sys
import pycdlib

src, out = sys.argv[1], sys.argv[2]
iso = pycdlib.PyCdlib()
iso.new(interchange_level=3, joliet=3, vol_ident="UNATTEND")
iso.add_file(src, "/AUTOUNAT.XML;1", joliet_path="/autounattend.xml")
iso.write(out)
iso.close()
print(out)
