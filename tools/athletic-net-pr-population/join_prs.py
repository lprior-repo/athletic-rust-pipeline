#!/usr/bin/env python3
"""Join bio PRs into the US-only workbook -> new xlsx with SchoolID + wide per-event PR columns.

Inputs:
  - US-only workbook (original, untouched)
  - bio_prs.jsonl (one line per athlete, from fetch_bios.py)
Output:
  - <workbook>-PRs.xlsx  (original columns + SchoolID + one column per event short + PR Matched)
  - coverage_report.txt
"""
import json, re, zipfile, os, sys, time

WB_IN  = "/home/lewis/Downloads/Athletic-2026-US-Boys-Grade11-All-Athletes-US-Only.xlsx"
BIO    = "/home/lewis/Downloads/athletic-bio-fetch/bio_prs.jsonl"
OUTDIR = "/home/lewis/Downloads/athletic-bio-fetch"
OUT    = "/home/lewis/Downloads/Athletic-2026-US-Boys-Grade11-All-Athletes-US-Only-PRs.xlsx"
REPORT = os.path.join(OUTDIR, "coverage_report.txt")

def colname(n):
    s = ""
    while n > 0:
        n, r = divmod(n - 1, 26)
        s = chr(65 + r) + s
    return s

def esc(s):
    return s.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;")

def cell(ref, val, numeric=False):
    if val is None or val == "":
        return ""                      # sparse: omit empty cells
    if numeric and str(val).isdigit():
        return f'<c r="{ref}"><v>{val}</v></c>'
    return f'<c r="{ref}" t="str"><v>{esc(str(val))}</v></c>'

def main():
    t0 = time.time()
    # ---- 1. load bio PRs ----
    bio = {}
    for line in open(BIO):
        line = line.strip()
        if not line:
            continue
        try:
            r = json.loads(line)
        except Exception:
            continue
        bio[r["aid"]] = r
    log(f"loaded {len(bio)} bio records")

    # ---- 2. read original sheet ----
    z = zipfile.ZipFile(WB_IN)
    xml = z.read("xl/worksheets/sheet1.xml").decode()
    sd_i = xml.index("<sheetData>")
    head = xml[: sd_i + len("<sheetData>")]
    end_i = xml.index("</sheetData>") + len("</sheetData>")
    tail = xml[end_i:]

    CELL = re.compile(r'<c r="([A-Z]+)\d+"[^>]*>(.*?)</c>', re.S)
    ROW = re.compile(r"<row ([^>]*?)>(.*?)</row>", re.S)
    V = re.compile(r"<v>(.*?)</v>", re.S)

    rows = []
    freq = {}
    for m in ROW.finditer(xml[sd_i:end_i]):
        attrs, body = m.group(1), m.group(2)
        rm = re.search(r'r="(\d+)"', attrs)
        if not rm:
            continue
        rn = int(rm.group(1))
        vals = {}
        for cm in CELL.finditer(body):
            vm = V.search(cm.group(2))
            vals[cm.group(1)] = vm.group(1) if vm else ""
        row = [vals.get(c, "") for c in "ABCDEFGHI"]
        if rn > 1:
            for sh in row[5].split("; "):
                if sh:
                    freq[sh] = freq.get(sh, 0) + 1
        rows.append((rn, row))

    shorts = [s for s, _ in sorted(freq.items(), key=lambda kv: -kv[1])]
    ncols = 9 + 1 + len(shorts) + 1
    spans = f"1:{ncols}"
    nrows = len(rows)
    log(f"workbook: {nrows} rows, {len(shorts)} event shorts -> {ncols} output columns")

    head = re.sub(r'<dimension ref="[^"]*"/>',
                  f'<dimension ref="A1:{colname(ncols)}{nrows}"/>', head)

    NUM = {1, 3, 7, 10, ncols}          # AthleteID, GradeID, Result Count, SchoolID, PR Matched

    def emit(rn, vals, header=False):
        cs = [cell(colname(i) + str(rn), v, numeric=(i in NUM))
              for i, v in enumerate(vals, start=1)]
        body = "".join(cs)
        if header:
            return f'<row r="{rn}" spans="{spans}" s="1" customFormat="1" x14ac:dyDescent="0.25">{body}</row>'
        return f'<row r="{rn}" spans="{spans}" x14ac:dyDescent="0.25">{body}</row>'

    # ---- 3. emit ----
    buf = [head]
    m_ev = t_ev = with_school = with_pr = 0
    per_event = {}
    for rn, row in rows:
        if rn == 1:
            buf.append(emit(1, row + ["SchoolID"] + shorts + ["PR Matched"], header=True))
            continue
        aid = row[0]
        b = bio.get(aid)
        evlist = [s for s in row[5].split("; ") if s]
        t_ev += len(evlist)
        sid = ""
        mcount = 0
        if b:
            if b.get("school") is not None:
                sid = str(b["school"]); with_school += 1
            hit = False
            for s in evlist:
                if b["ev"].get(s, {}).get("pr"):
                    mcount += 1; hit = True
                    per_event[s] = per_event.get(s, 0) + 1
            if hit:
                with_pr += 1
        m_ev += mcount
        vals = row + [sid] + \
               [(b["ev"].get(s, {}).get("pr") or "") if b else "" for s in shorts] + \
               [str(mcount)]
        buf.append(emit(rn, vals))

    buf.append(tail)
    newxml = "".join(buf)

    # ---- 4. write output xlsx (copy all other parts) ----
    with zipfile.ZipFile(OUT, "w", zipfile.ZIP_DEFLATED) as zo:
        for item in z.infolist():
            if item.filename == "xl/worksheets/sheet1.xml":
                zo.writestr(item, newxml.encode())
            else:
                zo.writestr(item, z.read(item.filename))
    z.close()
    log(f"wrote {OUT} ({os.path.getsize(OUT)/1e6:.1f} MB) in {time.time()-t0:.1f}s")

    # ---- 5. report ----
    lines = []
    lines.append(f"rows in workbook          : {nrows-1}")
    lines.append(f"bio records loaded        : {len(bio)}")
    lines.append(f"athletes with SchoolID    : {with_school}")
    lines.append(f"athletes with >=1 PR      : {with_pr}")
    lines.append(f"event slots (Events col)  : {t_ev}")
    lines.append(f"event slots matched to PR : {m_ev}  ({100.0*m_ev/t_ev:.1f}%)")
    if nrows > 1:
        lines.append(f"avg events/athlete        : {t_ev/(nrows-1):.2f}")
    lines.append("")
    lines.append("per-event matched / listed (top 30):")
    for s in shorts[:30]:
        lines.append(f"  {s:22s} {per_event.get(s,0):7d} / {freq[s]:7d}  ({100.0*per_event.get(s,0)/freq[s]:5.1f}%)")
    rep = "\n".join(lines)
    open(REPORT, "w").write(rep + "\n")
    print(rep)

def log(m):
    sys.stderr.write(f"[join] {m}\n"); sys.stderr.flush()

if __name__ == "__main__":
    main()
