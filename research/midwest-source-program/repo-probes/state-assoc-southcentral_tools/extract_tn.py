#!/usr/bin/env python3
"""Extract the TSSAA member-school directory embedded in portal.tssaa.org/common/directory/.

Input : samples/tn-portal-directory.html
Output: samples/tn-schools-extracted.tsv   (tssaa_school_id, school_name_raw)
"""
import re

h = open("samples/tn-portal-directory.html", encoding="utf-8", errors="replace").read()
m = re.search(r"source:\s*(\[.*?\])\s*,\s*updater", h, re.S) or re.search(r"source:\s*(\[.*\])", h, re.S)
items = re.findall(r"\{id:\s*'(\d+)',\s*name:\s*'((?:[^'\\]|\\.)*)'\}", m.group(1))
with open("samples/tn-schools-extracted.tsv", "w", encoding="utf-8") as f:
    f.write("tssaa_school_id\tschool_name_raw\n")
    for i, n in items:
        f.write(f"{i}\t{n}\n")
print("entries:", len(items))
