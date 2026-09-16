#!/usr/bin/env python3
"""Exercise the real exhaustive CLI against isolated synthetic HTTP/XLSX fixtures."""

import argparse
import csv
import html
import json
import signal
import subprocess
import tempfile
import threading
import time
import urllib.error
import urllib.request
import zipfile
from contextlib import ExitStack
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

HEADERS = (
    "Person First", "Person Last", "Person Email",
    "Address Mailing / Permanent Street Combined",
    "Address Mailing / Permanent City", "Address Mailing / Permanent Region",
    "Address Mailing / Permanent Postal", "Sports Created Date", "Sports Sport",
    "Sports Rating", "Origin Source Date", "Origin Source", "Schools Name",
)
PERSON = dict(zip(HEADERS, (
    "Ada", "Runner", "private@example.invalid", "1 Private Example Way",
    "Austin", "TX", "00000", "2026-01-01", "Basketball", "0",
    "2026-01-02", "Synthetic source", "Central High School",
)))
BLANK = dict(PERSON, **{"Person First": "", "Person Last": ""})
PRIVATE_VALUES = (PERSON["Person Email"], PERSON[HEADERS[3]], PERSON[HEADERS[6]])


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


def worksheet(rows):
    values = [list(HEADERS)] + [[row.get(key, "") for key in HEADERS] for row in rows]
    rendered = []
    for row_number, row in enumerate(values, 1):
        cells = "".join(
            f'<c r="{chr(65 + column)}{row_number}" t="inlineStr"><is>'
            f'<t xml:space="preserve">{html.escape(value)}</t></is></c>'
            for column, value in enumerate(row)
        )
        rendered.append(f'<row r="{row_number}">{cells}</row>')
    return ('<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">'
            '<sheetData>' + ''.join(rendered) + '</sheetData></worksheet>')


def workbook(path, rows):
    with zipfile.ZipFile(path, 'w', zipfile.ZIP_DEFLATED) as archive:
        archive.writestr('[Content_Types].xml',
            '<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">'
            '<Default Extension="xml" ContentType="application/xml"/>'
            '<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>'
            '<Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>'
            '</Types>')
        archive.writestr('xl/workbook.xml',
            '<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" '
            'xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">'
            '<sheets><sheet name="Export" sheetId="1" r:id="rId1"/>'
            '<sheet name="Ignored" sheetId="2" r:id="rId2"/></sheets></workbook>')
        archive.writestr('xl/_rels/workbook.xml.rels',
            '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">'
            + ''.join(f'<Relationship Id="rId{i}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet{i}.xml"/>' for i in (1, 2))
            + '</Relationships>')
        archive.writestr('xl/worksheets/sheet1.xml', worksheet(rows + [{}]))
        archive.writestr('xl/worksheets/sheet2.xml', worksheet([PERSON]))


def model_body(value):
    return json.dumps({'choices': [{'message': {'content': json.dumps(value)}}]}).encode()


class Service:
    def __init__(self, kind, status=200, delay=0, incomplete=False, malformed=False, upstream=None):
        self.kind, self.status, self.delay = kind, status, delay
        self.incomplete, self.malformed, self.upstream = incomplete, malformed, upstream
        self.requests, self.filters, self.private_payloads = 0, set(), 0
        self.address_payloads, self.transient_failures = 0, 0
        self.lock, self.seen = threading.Lock(), threading.Event()

    def response(self, path, body):
        with self.lock:
            self.requests += 1
            forbidden = PRIVATE_VALUES if self.kind == 'search' else PRIVATE_VALUES[:1]
            if any(value.encode() in body for value in forbidden):
                self.private_payloads += 1
                return 400, b'{"error":"private fields reached transport"}'
            if self.kind != 'search' and all(value.encode() in body for value in PRIVATE_VALUES[1:]):
                self.address_payloads += 1
            transient = self.transient_failures > 0
            if transient:
                self.transient_failures -= 1
            payload = json.loads(body)
            if self.kind == 'search':
                self.filters.add(payload.get('fq'))
        delay = self.delay
        self.seen.set()
        time.sleep(delay)
        if transient:
            return 503, b'{"error":"transient fixture failure"}'
        if self.status != 200:
            return self.status, b'{"error":"controlled service failure"}'
        if path != ('/' if self.kind == 'search' else '/v1/chat/completions'):
            return 404, b'{}'
        if self.upstream:
            request = urllib.request.Request(self.upstream.rstrip('/') + path, data=body,
                                             headers={'Content-Type': 'application/json'})
            try:
                with urllib.request.urlopen(request, timeout=90) as response:
                    return response.status, response.read(65537)
            except urllib.error.HTTPError as error:
                return error.code, error.read(65537)
        if self.kind == 'search':
            sport = 'cross-country' if 'a:xc' in payload['fq'] else 'track-and-field'
            label = 'Cross Country' if sport == 'cross-country' else 'Track & Field'
            def row(identifier):
                return (f'<tr><td><a href="/athlete/{identifier}/{sport}">Ada Runner</a> '
                        f'Central High School, Austin TX. Class of 2027. Sport: {label}</td></tr>')
            result = row(7)
            count = 1
            if self.incomplete:
                result = ''.join(row(i) for i in range(1, 166)) + row('') + row('')
                count = 167
            return 200, json.dumps({'d': {'count': count, 'results': result, 'pager': ''}}).encode()
        if self.malformed:
            return 200, model_body({})
        if self.kind == 'extractor':
            sport = 'Cross Country' if b'/cross-country' in body else 'Track & Field'
            return 200, model_body({'athlete_name': 'Ada Runner', 'school': 'Central High School',
                'location': 'Austin TX', 'graduation_year': 2027, 'sports': [sport], 'marks': None})
        return 200, model_body({'decision': 'MATCH', 'candidate_index': 0, 'confidence': 0.99,
            'track_confirmed': True, 'xc_confirmed': True, 'reason': 'Independent identity evidence agrees',
            'model_status': 'ok'})

    def __enter__(self):
        service = self
        class Handler(BaseHTTPRequestHandler):
            def do_POST(self):
                body = self.rfile.read(int(self.headers.get('Content-Length', '0')))
                try:
                    status, response = service.response(self.path, body)
                    self.send_response(status)
                    self.send_header('Content-Length', str(len(response)))
                    self.send_header('Content-Type', 'application/json')
                    self.end_headers()
                    self.wfile.write(response)
                except (BrokenPipeError, ConnectionResetError):
                    pass
            def log_message(self, *_args):
                pass
        self.server = ThreadingHTTPServer(('127.0.0.1', 0), Handler)
        self.thread = threading.Thread(target=self.server.serve_forever)
        self.thread.start()
        self.url = f'http://127.0.0.1:{self.server.server_port}'
        return self

    def __exit__(self, *_args):
        self.server.shutdown()
        self.server.server_close()
        self.thread.join(timeout=5)
        require(not self.thread.is_alive(), 'fixture server failed to stop')


def configure(path, search, extractor, reviewer, args):
    path.write_text(f'''[workbook]
sports = ["Track and Field: Mens"]
expected_graduation_year = 2027
[discovery]
athletic_search_url = "{search.url}"
max_candidates = 10
request_timeout_seconds = 10
search_delay_ms = 500
max_attempts = 3
circuit_breaker_threshold = 3
ambiguity_margin = 0.03
[retrieval]
authorized_direct_fetch = false
respect_robots_txt = true
delay_ms = 500
user_agent = "ExhaustiveScenario/1.0"
page_text_limit = 60000
[ollama]
api = "openai-compatible"
enabled = true
url = "{extractor.url}"
model = "{args.q5_model}"
timeout_seconds = 90
[identity_review]
api = "openai-compatible"
enabled = true
url = "{reviewer.url}"
model = "{args.q4_model}"
timeout_seconds = 90
[matching]
match_threshold = 0.93
close_threshold = 0.86
review_threshold = 0.75
require_corroboration = true
''')


def command(args, source, config, output, maximum=None):
    result = [str(args.binary), 'run', '--input', str(source), '--config', str(config),
              '--out-dir', str(output), '--all-workbook-rows', '--first-worksheet-only']
    if args.no_ai:
        result.append('--no-ai')
    return result + ([] if maximum is None else ['--max', str(maximum)])


def collect(process, output, started):
    try:
        stdout, stderr = process.communicate(timeout=120)
    except subprocess.TimeoutExpired:
        process.kill()
        stdout, stderr = process.communicate()
        raise RuntimeError(f'CLI timeout; stderr={stderr}')
    evidence = {'command': process.args, 'exit': process.returncode, 'stdout': stdout,
                'stderr': stderr, 'seconds': time.perf_counter() - started}
    (output.parent / f'invocation-{process.pid}.json').write_text(json.dumps(evidence, indent=2))
    return evidence


def invoke(args, source, config, output, maximum=None):
    started = time.perf_counter()
    process = subprocess.Popen(command(args, source, config, output, maximum),
        stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    return collect(process, output, started)


def records(output):
    return [json.loads(line) for line in (output / 'matches.jsonl').read_text().splitlines()]


def coverage(output, completed, pending, retryable):
    report = json.loads((output / 'coverage.json').read_text())
    require(report['total_rows'] == 2 and report['sheets'] == ['Export'], 'wrong population')
    require((report['completed_rows'], report['pending_rows'], report['retryable_rows'])
            == (completed, pending, retryable), f'wrong coverage: {report}')
    require(report['complete'] == (completed == 2 and pending == 0 and retryable == 0),
            'completion flag disagrees with outcomes')
    return report


def check_positive(output):
    result = records(output)
    require([row['status'] for row in result] == ['MATCH', 'INPUT_ERROR'],
            f'positive outcomes incorrect: {[(r["status"], r["ai_logic"]) for r in result]}')
    require(result[0]['deterministic_decision'] is not None, 'deterministic first pass missing')
    require(result[0]['track_confirmed'] and result[0]['xc_confirmed'], 'both sports not attributed')
    require(result[0]['prospect']['source_fields'] == PERSON, 'JSONL source fields changed')
    with (output / 'matches.csv').open(newline='') as stream:
        first = next(csv.DictReader(stream))
        require({key: first[key] for key in HEADERS} == PERSON, 'CSV source fields changed')
    return coverage(output, 2, 0, 0)


def scenario(args, root, name):
    directory = root / name
    directory.mkdir()
    source, config, output = directory / 'input.xlsx', directory / 'config.toml', directory / 'out'
    workbook(source, [BLANK, PERSON] if name == 'cancellation' else [PERSON, BLANK])
    with ExitStack() as stack:
        search = stack.enter_context(Service('search', status=503 if name == 'search_503' else 200,
            incomplete=name == 'incomplete', delay=3 if name == 'cancellation' else args.search_latency))
        extractor = stack.enter_context(Service('extractor', status=503 if name == 'model_503' else 200,
            malformed=name == 'model_schema', upstream=args.actual_q5_url if args.actual_models else None))
        reviewer = stack.enter_context(Service('reviewer', upstream=args.actual_q4_url if args.actual_models else None))
        services = (search, extractor, reviewer)
        if name == 'transient':
            search.transient_failures = 1
            extractor.transient_failures = 2
        configure(config, *services, args)
        if name == 'cancellation':
            seeded = invoke(args, source, config, output, 1)
            require(seeded['exit'] == 0, seeded['stderr'])
            before = (output / 'checkpoint.jsonl').read_bytes()
            started = time.perf_counter()
            process = subprocess.Popen(command(args, source, config, output), stdin=subprocess.DEVNULL,
                stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
            try:
                require(search.seen.wait(10), 'processing never reached search')
                locked = invoke(args, source, config, output)
                require(locked['exit'] != 0 and 'another process' in locked['stderr'], 'writer lock failed')
                search.delay = 0
                process.send_signal(signal.SIGINT)
                interrupted = collect(process, output, started)
                require(interrupted['exit'] != 0, 'cancelled invocation claimed success')
            finally:
                if process.poll() is None:
                    process.kill()
                    process.communicate()
            require((output / 'checkpoint.jsonl').read_bytes() == before, 'cancel lost prior or committed active row')
            coverage(output, 1, 1, 0)
            result = invoke(args, source, config, output)
            require(result['exit'] == 0, result['stderr'])
            require([row['status'] for row in records(output)] == ['INPUT_ERROR', 'MATCH'], 'cancel resume failed')
            report = coverage(output, 2, 0, 0)
        else:
            result = invoke(args, source, config, output)
            if name in ('positive', 'transient'):
                require(result['exit'] == 0, result['stderr'])
                report = check_positive(output)
                require(search.filters == {'t:a a:tf', 't:a a:xc'}, 'missing sport search')
                if args.no_ai:
                    require(extractor.requests == reviewer.requests == 0, 'no-AI mode contacted models')
                    require(records(output)[0]['model_decision']['model_status'] == 'not_run_deterministic', 'no-AI provenance missing')
                else:
                    require(extractor.requests >= 2 and reviewer.requests >= 1, 'both models not exercised')
                    require(all(s.address_payloads == s.requests for s in (extractor, reviewer)), 'full address missing from model request')
                before = [service.requests for service in services]
                for service in services:
                    service.status = 503
                resumed = invoke(args, source, config, output)
                require(resumed['exit'] == 0, resumed['stderr'])
                require(before == [service.requests for service in services], 'completed resume made external requests')
                check_positive(output)
                (output / 'checkpoint.jsonl').unlink()
                cached = invoke(args, source, config, output)
                require(cached['exit'] == 0, cached['stderr'])
                require(before == [service.requests for service in services], 'cache-only resume made external requests')
                check_positive(output)
            else:
                expected = 'AI_ERROR' if name.startswith('model_') else 'SEARCH_ERROR'
                require(result['exit'] != 0, 'service failure claimed success')
                rows = records(output)
                require(len(rows) == 1 and rows[0]['status'] == expected, f'failure was not {expected}')
                require(rows[0]['model_decision']['decision'] == 'ERROR', 'failure became identity rejection')
                require(not rows[0]['selected_profile_url'], 'error retained attribution')
                report = coverage(output, 0, 1, 1)
        require(all(service.private_payloads == 0 for service in services), 'private fields reached transport')
        return {'name': name, 'exit': result['exit'], 'seconds': round(result['seconds'], 4),
                'requests': dict(zip(('search', 'extractor', 'reviewer'), (s.requests for s in services))),
                'completed': report['completed_rows'], 'pending': report['pending_rows'],
                'retryable': report['retryable_rows']}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--actual-models', action='store_true')
    parser.add_argument('--actual-q5-url', default='http://127.0.0.1:11000')
    parser.add_argument('--actual-q4-url', default='http://127.0.0.1:11001')
    parser.add_argument('--q5-model', default='Qwen3.6-35B-A3B-UD-Q5_K_XL.gguf')
    parser.add_argument('--q4-model', default='Qwen3.6-35B-A3B-UD-Q4_K_XL.gguf')
    parser.add_argument('--search-latency', type=float, default=0)
    parser.add_argument('--positive-only', action='store_true')
    parser.add_argument('--no-ai', action='store_true')
    args = parser.parse_args()
    args.binary = args.binary.resolve()
    root = Path(tempfile.mkdtemp(prefix='athletic-cli-evidence-'))
    names = ['positive'] if args.actual_models or args.positive_only else (
        ['positive', 'transient', 'search_503', 'incomplete', 'cancellation'] if args.no_ai else
        ['positive', 'transient', 'model_503', 'model_schema', 'search_503', 'incomplete', 'cancellation'])
    outcomes = []
    try:
        for name in names:
            outcomes.append(scenario(args, root, name))
        print(json.dumps({'passed': True, 'evidence': str(root), 'scenarios': outcomes}))
        return 0
    except Exception as error:
        print(json.dumps({'passed': False, 'evidence': str(root), 'error': str(error), 'scenarios': outcomes}))
        return 1


if __name__ == '__main__':
    raise SystemExit(main())
