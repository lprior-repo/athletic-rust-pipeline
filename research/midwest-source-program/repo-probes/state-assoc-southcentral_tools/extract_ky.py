#!/usr/bin/env python3
"""Extract KHSAA member-school ArbiterLive entity ids.

Input : samples/ky-member-school-directory.html
Output: samples/ky-schools-arbiterlive.tsv   (arbiterlive_entity_id, school_name)
"""
import html
import re

h = open("samples/ky-member-school-directory.html", encoding="utf-8", errors="replace").read()
seen = {}
for u, n in re.findall(r"<a href='(https://www\.arbiterlive\.com/Teams\?entityId=\d+)'>([^<]{2,80})</a>", h):
    seen.setdefault(u, html.unescape(n).strip())
with open("samples/ky-schools-arbiterlive.tsv", "w", encoding="utf-8") as f:
    f.write("arbiterlive_entity_id\tschool_name\n")
    for u, n in sorted(seen.items(), key=lambda kv: int(re.search(r"entityId=(\d+)", kv[0]).group(1))):
        f.write(f"{re.search(r'entityId=(\d+)', u).group(1)}\t{n}\n")
print("schools:", len(seen))
