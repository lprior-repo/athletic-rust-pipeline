#!/usr/bin/env python3
import zipfile, re, shutil, sys
from collections import Counter, defaultdict

SRC = '/home/lewis/Downloads/Athletic-2026-US-Boys-Grade11-All-Athletes (1).xlsx'
DST = '/home/lewis/Downloads/Athletic-2026-US-Boys-Grade11-All-Athletes-US-Only.xlsx'

US_STATES = {'AL','AK','AZ','AR','CA','CO','CT','DE','FL','GA','HI','ID','IL','IN','IA','KS','KY',
             'LA','ME','MD','MA','MI','MN','MS','MO','MT','NE','NV','NH','NJ','NM','NY','NC','ND',
             'OH','OK','OR','PA','RI','SC','SD','TN','TX','UT','VT','VA','WA','WV','WI','WY','DC'}

# Non-US state tokens that appear in the States column (overseas/foreign/territory).
NON_US_STATE_TOKENS = {'AE','AP','GU','PR','VI','AS','MP','ON','GUAM','TOKYO','YAMAGUCHI','GYEONGGI-DO'}

# Team-name substrings (matched case-insensitively) that mark an overseas/international athlete.
INTL_KEYWORDS = [
    # DoD / overseas military-base schools
    'stuttgart','patch','n.c. kinnick','kinnick','kadena','vicenza','wiesbaden','yokota','ramstein',
    'zama american','naples','spangdahlem','rota','vilseck','shape','osan','sigonella','lakenheath',
    'daegu','edgren','e.j. king','hohenfels','baumholder','brussels american','afnorth',
    # international schools
    'international','american school','american cs','american overseas','seoul','korea',
    'zurich','munich','singapore','cobham','in london','rome','duesseldorf','dusseldorf',
    'beijing','chadwick','jeju','belgium','hague','black forest academy','frankfurt','okinawa',
    'morrison taipei','taipei','vienna',
    # foreign country schools / clubs
    'jamaica','greater portmore','st. andrew technical','old harbour','lawrence tavern','cayman',
    'mustang track club','vision academy','central qld','unisc','ecole rose','mayne harriers',
    'upper lockyer','ipswich & district','springwood school','balmoral lac','guyra',
    # territories (Guam public schools + PR)
    'guam','simon sanchez','tiyan','john f. kennedy','george washington','puerto rico',
    # ambiguous single-token Guam school name (empty state + no other signal)
    # 'southern' handled separately below to avoid US false positives
]

def classify(team_lower):
    # 'southern' -> Guam Southern High ONLY when it is the whole team string
    if team_lower.strip() == 'southern':
        return 'intl_territory'
    for kw in INTL_KEYWORDS:
        if kw in team_lower:
            return 'intl'
    return None

z = zipfile.ZipFile(SRC)
sheet = z.read('xl/worksheets/sheet1.xml').decode('utf-8')
m = re.search(r'<sheetData>.*?</sheetData>', sheet, re.S)
sd = m.group(0)
rows = re.findall(r'<row .*?</row>', sd, re.S)

header_row = rows[0]
data_rows = rows[1:]
print(f'header + {len(data_rows)} data rows')

def parse_row(rowxml):
    """Return dict col->value for a data row."""
    vals = {}
    for cm in re.finditer(r'<c r="([A-Z])\d+"(?:[^>]*?)>(?:<v>(.*?)</v>)?</c>', rowxml, re.S):
        col, v = cm.group(1), cm.group(2)
        vals[col] = v if v is not None else ''
    return vals

kept_rows = []
removed = []  # (rowxml, category, team, states)
counts = Counter()

for row in data_rows:
    p = parse_row(row)
    states_raw = (p.get('E') or '').strip()
    team_raw = (p.get('D') or '').strip()
    team_lower = team_raw.lower()
    # tokenize states
    toks = [t.strip().upper() for t in re.split(r'[;,]', states_raw) if t.strip()]
    has_us = any(t in US_STATES for t in toks)
    has_nonus = any(t in NON_US_STATE_TOKENS for t in toks)
    if has_us:
        kept_rows.append(row); counts['keep_us_state'] += 1; continue
    # no valid US state
    cat = None
    if has_nonus:
        cat = 'intl_state_token'
    else:
        cat = classify(team_lower)
    if cat is not None:
        removed.append((row, cat, team_raw, states_raw)); counts[cat] += 1
    else:
        # not international: unattached / US club / US school missing state
        if 'unattached' in team_lower or team_lower.startswith('unat') or 'unat-' in team_lower:
            counts['keep_unattached'] += 1
        elif any(t in team_lower for t in ['track club','athletics','elite','striders','stars','outlaws','bulldogs','venom','tc','sports','parasport','harriers','lac','thundering herd','primal','unified']):
            counts['keep_us_club'] += 1
        else:
            counts['keep_missing_state'] += 1
        kept_rows.append(row)

print(f'kept: {len(kept_rows)}   removed: {len(removed)}')
print('--- category counts (kept buckets) ---')
for k in ['keep_us_state','keep_unattached','keep_us_club','keep_missing_state']:
    print(f'  {k:22} {counts.get(k,0)}')
print('--- category counts (removed) ---')
for k in ['intl_state_token','intl']:
    print(f'  {k:22} {counts.get(k,0)}')
# finer breakdown of removed by team string
removed_teams = Counter((cat, t) for _, cat, t, _ in removed)
print('--- removed team strings ---')
for (cat, t), c in sorted(removed_teams.items(), key=lambda x: -x[1]):
    print(f'  {c:4}  [{cat}]  {t!r}')

# ---- rebuild sheetData with kept rows (renumbered) ----
new_rows = [header_row]
n = 1
for row in kept_rows:
    n += 1
    new = re.sub(r'r="([A-Z])(\d+)"', lambda mm: f'r="{mm.group(1)}{n}"', row)
    new = re.sub(r'<row r="\d+"', f'<row r="{n}"', new, count=1)
    new_rows.append(new)
new_sd = '<sheetData>' + ''.join(new_rows) + '</sheetData>'
new_sheet = sheet[:m.start()] + new_sd + sheet[m.end():]

# write new zip preserving everything else
with zipfile.ZipFile(DST, 'w', zipfile.ZIP_DEFLATED) as zout:
    for item in z.infolist():
        data = z.read(item.filename)
        if item.filename == 'xl/worksheets/sheet1.xml':
            data = new_sheet.encode('utf-8')
        zout.writestr(item, data)
print(f'wrote {DST}')
print(f'final rows (incl header): {len(new_rows)}')
