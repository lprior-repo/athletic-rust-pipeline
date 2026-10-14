from pathlib import Path
import hashlib, json, re

ROOT = Path(__file__).resolve().parent
errors = []
checks = []

def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def check(condition, description):
    checks.append({'check': description, 'passed': bool(condition)})
    if not condition:
        errors.append(description)

pins = ROOT / 'artifact-pins.sha256'
if pins.exists():
    pinned = 0
    for line in pins.read_text().splitlines():
        expected, rel = line.split('  ', 1)
        path = ROOT / rel
        check(path.is_file() and sha(path) == expected, 'artifact pin: ' + rel)
        pinned += 1
else:
    pinned = 0
    check(False, 'artifact-pins.sha256 exists')
for state in ['candidate-first', 'candidate-final']:
    entries = json.loads((ROOT / state / 'source-files.json').read_text())
    check(len(entries) == 22, state + ' contains exactly 22 source paths')
    for entry in entries:
        path = ROOT / state / 'source' / entry['path']
        check(path.is_file() and sha(path) == entry['sha256'] and path.stat().st_size == entry['bytes'], state + ' source: ' + entry['path'])
reviewed=json.loads((ROOT/'review/reviewed-files.json').read_text())['files']
final_pins={r['path']:r['sha256'] for r in json.loads((ROOT/'candidate-final/source-files.json').read_text())}
check(len(reviewed)==22 and {r['path'] for r in reviewed}==set(final_pins), 'final review covers exactly the 22 candidate paths')
for row in reviewed:
    check(row['reviewed_live_sha256']==final_pins[row['path']], 'final review source hash: '+row['path'])
commit=json.loads((ROOT/'commit-coverage.json').read_text())
check(commit['changed_path_count']==22 and {r['path'] for r in commit['files']}==set(final_pins), 'source commit covers exactly 22 tested paths')
for row in commit['files']:
    check(row['sha256']==final_pins[row['path']], 'source commit hash: '+row['path'])
raw1 = (ROOT / 'inputs/baseline-probe/legacy-bytes.json').read_bytes()
raw2 = (ROOT / 'inputs/baseline-probe-v2/legacy-bytes.json').read_bytes()
fixture = (ROOT / 'candidate-final/source/crates/census-service/src/census/sweep/roster/journal/legacy_roster_bytes.json').read_bytes()
check(fixture == raw2 + b'\n', 'installed golden is exact V2 output plus one terminal LF')
check(json.loads(fixture) == json.loads(raw2), 'LF normalization leaves all JSON data identical')
check(len(json.loads(json.loads(raw1)['canonical_bytes'])[1]) == 0, 'V1 empty-team limitation retained')
check(len(json.loads(json.loads(raw2)['canonical_bytes'])[1]) == 2, 'V2 contains two canonical teams')
for path in ROOT.rglob('*'):
    if path.is_file() and path.suffix in ['.json', '.jsonl'] and path.name != 'integrity-check.json':
        try:
            if path.suffix == '.json':
                json.loads(path.read_text())
            else:
                for line in path.read_text().splitlines():
                    if line.strip(): json.loads(line)
        except (ValueError, UnicodeError) as exc:
            errors.append(f'JSON parsing: {path.relative_to(ROOT)}: {exc}')
ledger=json.loads((ROOT/'execution-ledger.json').read_text())['commands']
for row in ledger:
    path=ROOT/row['portable_raw_log']
    check(path.is_file() and sha(path)==row['log_sha256'], 'raw log unchanged: '+row['lane'])
    if row['test_results']:
        check(len(row['test_results'])==row['observed_result_rows'], 'selected count: '+row['lane'])
        check(len({(r['binary'],r['test']) for r in row['test_results']})==len(row['test_results']), 'no duplicate selected result: '+row['lane'])
    if row['evidence_status']=='PASS':
        check(row.get('outer_exit')==0 and row['terminal_moon_success'], 'PASS has terminal log and attributed zero exit: '+row['lane'])
current=set()
for row in ledger:
    if row['source_state']=='candidate-final' and row['evidence_status']=='PASS':
        current.update((r['binary'],r['test']) for r in row['test_results'])
check(len(current)==json.loads((ROOT/'test-overlap.json').read_text())['final_distinct_tests'], 'final distinct test count excludes old probes and first run')
for name in ['REPORT.md','assurance-bundle.md','commands.md','baseline-history.md','qualification-boundary.md','final-evidence-decision.md','truth-serum-report.md']:
    text=(ROOT/name).read_text()
    for match in re.finditer(r'\[[^\]]+\]\(([^)]+)\)',text):
        link=match.group(1).split('#',1)[0]
        if link and not re.match(r'^[a-zA-Z][a-zA-Z0-9+.-]*:',link):
            check((ROOT/link).exists(), 'authored relative link: '+name+' -> '+link)
result={'status':'PASS' if not errors else 'FAIL','meaning':'Mechanical detached-bundle integrity only; not active-context truth-serum, source-code correctness or release approval.','pinned_files':pinned,'check_count':len(checks),'failed_checks':errors,'checks':checks}
print(json.dumps(result,indent=2))
raise SystemExit(1 if errors else 0)
