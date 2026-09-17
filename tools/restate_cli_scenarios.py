#!/usr/bin/env python3
"""Real Restate CLI verification; never launches a worker or Restate server.

Main must supervise all long-lived processes, including fixtures, with hub:
  tools/restate_cli_scenarios.py serve-fixtures --root /tmp/restate-proof --port 19180
  # Start/register the worker externally using /tmp/restate-proof/config.toml.
  tools/restate_cli_scenarios.py check --root /tmp/restate-proof --binary PATH --restate-url URL
  tools/restate_cli_scenarios.py restart-case --root /tmp/restate-proof --binary PATH --restate-url URL --kind worker --phase prepare
  # Immediately kill/restart the worker externally, preserving Restate storage.
  tools/restate_cli_scenarios.py restart-case --root /tmp/restate-proof --binary PATH --restate-url URL --kind worker --phase verify

Repeat restart-case with --kind server, restarting the persistent server instead.
Prepare runs --max 0 to establish bindings/fingerprint, then starts a finite CLI
request. Once the second distinct extractor request is in flight it terminates
ONLY that CLI and returns. The accepted invocation remains owned by Restate.
The synthetic second request pauses for 65 seconds (below model timeout).
Verify must start within that window; it asserts the original request remained
in flight when verification began. Fixture processes must survive both phases.
Use a fresh root and fresh Restate namespace/storage for each complete run.
Control GET/POST /control reports cumulative counters; POST changes settings,
never counters. Raw CLI and fixture request logs are retained under root.
"""

import argparse
import csv
import json
import subprocess
import threading
import time
import urllib.request
from contextlib import ExitStack
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

import exhaustive_cli_scenarios as base

require = base.require
ROLES = ('search', 'extractor', 'reviewer')


def write(path, value):
    path.write_text(json.dumps(value, indent=2) + '\n')


def request(url, value=None):
    data = None if value is None else json.dumps(value).encode()
    req = urllib.request.Request(url, data=data, headers={'Content-Type': 'application/json'})
    with urllib.request.urlopen(req, timeout=15) as response:
        return json.load(response)


class Fixture(base.Service):
    def __init__(self, kind, root):
        super().__init__(kind)
        self.root = root
        self.blocked = False
        self.candidate_status = {}
        self.candidate_delay = {}
        self.candidates = {}
        self.inflight = {}
        self.provenance_violations = 0
        self.audit_lock = threading.Lock()

    def response(self, path, body):
        candidate = 'xc' if b'/cross-country' in body else 'tf'
        with self.lock:
            if self.kind == 'extractor':
                self.candidates[candidate] = self.candidates.get(candidate, 0) + 1
                self.inflight[candidate] = self.inflight.get(candidate, 0) + 1
            provenance = self.kind != 'search' and b'restate-fixture:' in body
            self.provenance_violations += int(provenance)
            blocked = self.kind == 'search' and self.blocked and b'Blocked' in body
            forced = self.candidate_status.get(candidate, 200) if self.kind == 'extractor' else 200
            delay = self.candidate_delay.get(candidate, 0) if self.kind == 'extractor' else 0
        event = {'time': time.time(), 'role': self.kind, 'path': path,
                 'candidate': candidate if self.kind == 'extractor' else None,
                 'body': body.decode(errors='replace')}
        with self.audit_lock:
            with (self.root / ('raw-' + self.kind + '.jsonl')).open('a') as stream:
                stream.write(json.dumps(event) + '\n')
        try:
            # Always use the existing hook for counting, privacy checks, and schema.
            status, response = super().response(path, body)
            time.sleep(delay)
            if provenance:
                return 400, b'{"error":"source provenance reached model"}'
            if blocked or forced != 200:
                return (503 if blocked else forced), b'{"error":"conditional fixture failure"}'
            return status, response
        finally:
            if self.kind == 'extractor':
                with self.lock:
                    self.inflight[candidate] -= 1

    def snapshot(self):
        with self.lock:
            return {'requests': self.requests, 'candidates': dict(self.candidates),
                    'inflight': dict(self.inflight), 'private_payloads': self.private_payloads,
                    'provenance_violations': self.provenance_violations,
                    'address_payloads': self.address_payloads, 'max_active': self.max_active,
                    'filters': sorted(str(x) for x in self.filters)}


def serve(args):
    args.root.mkdir(parents=True, exist_ok=True)
    with ExitStack() as stack:
        services = {role: stack.enter_context(Fixture(role, args.root)) for role in ROLES}
        base.configure(args.root / 'config.toml', *(services[r] for r in ROLES),
                       argparse.Namespace(q5_model='fixture-q5', q4_model='fixture-q4'))

        class Control(BaseHTTPRequestHandler):
            def handle_control(self, update):
                if self.path != '/control':
                    self.send_error(404)
                    return
                try:
                    if update:
                        changes = json.loads(self.rfile.read(int(self.headers.get('Content-Length', '0'))))
                        allowed = {'status', 'delay', 'incomplete', 'malformed', 'blocked',
                                   'transient_failures', 'candidate_status', 'candidate_delay'}
                        require(set(changes) <= set(ROLES), 'unknown service')
                        for role, settings in changes.items():
                            require(set(settings) <= allowed, 'unknown setting')
                            with services[role].lock:
                                for key, value in settings.items():
                                    setattr(services[role], key, value)
                    payload = json.dumps({r: s.snapshot() for r, s in services.items()}).encode()
                    self.send_response(200)
                    self.send_header('Content-Type', 'application/json')
                    self.send_header('Content-Length', str(len(payload)))
                    self.end_headers()
                    self.wfile.write(payload)
                except (ValueError, RuntimeError) as error:
                    self.send_error(400, str(error))

            def do_GET(self):
                self.handle_control(False)

            def do_POST(self):
                self.handle_control(True)

            def log_message(self, *_args):
                pass

        server = ThreadingHTTPServer(('127.0.0.1', args.port), Control)
        metadata = {'control_url': f'http://127.0.0.1:{server.server_port}/control',
                    'config': str((args.root / 'config.toml').resolve()),
                    'services': {r: s.url for r, s in services.items()}}
        write(args.root / 'metadata.json', metadata)
        print(json.dumps({'ready': True, **metadata}), flush=True)
        try:
            server.serve_forever()
        finally:
            server.server_close()


def control(args, changes=None):
    metadata = json.loads((args.root / 'metadata.json').read_text())
    return request(metadata['control_url'], changes)


def reset(args):
    return control(args, {r: {'status': 200, 'delay': 0, 'incomplete': False,
                             'malformed': False, 'blocked': False, 'transient_failures': 0,
                             'candidate_status': {}, 'candidate_delay': {}} for r in ROLES})


def counts(snapshot):
    return {r: snapshot[r]['requests'] for r in ROLES}


def privacy(snapshot):
    for role in ROLES:
        require(snapshot[role]['private_payloads'] == 0, f'{role}: private payload')
        require(snapshot[role]['provenance_violations'] == 0, f'{role}: provenance leaked')
        require(snapshot[role]['max_active'] <= 1, f'{role}: global serialization failed')


def setup(args, name, rows=None):
    directory = args.root / name
    directory.mkdir(exist_ok=False)
    rows = [dict(row, **{'Origin Source': f'restate-fixture:{args.root.resolve()}:{name}:{i}'})
            for i, row in enumerate(rows if rows is not None else [base.PERSON, base.BLANK])]
    base.workbook(directory / 'input.xlsx', rows)
    write(directory / 'source.json', rows)
    return directory


def command(args, directory, output='out', extra=(), config=None):
    return [str(args.binary.resolve()), 'run-restate', '--input', str(directory / 'input.xlsx'),
            '--config', str(config or args.root / 'config.toml'), '--out-dir', str(directory / output),
            '--restate-url', args.restate_url, '--concurrency', '3',
            '--request-timeout-seconds', '120', '--first-worksheet-only',
            '--i-have-written-authorization', *extra]


def invoke(args, directory, output='out', extra=(), config=None):
    cmd = command(args, directory, output, extra, config)
    stamp = str(time.time_ns())
    stdout = directory / (stamp + '.stdout.log')
    stderr = directory / (stamp + '.stderr.log')
    started = time.monotonic()
    with stdout.open('w') as out, stderr.open('w') as err:
        process = subprocess.Popen(cmd, stdin=subprocess.DEVNULL, stdout=out, stderr=err)
        try:
            code = process.wait(timeout=240)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait(timeout=10)
            raise RuntimeError(f'CLI exceeded deadline; logs: {stdout}, {stderr}')
    evidence = {'command': cmd, 'exit': code, 'stdout': str(stdout), 'stderr': str(stderr),
                'seconds': time.monotonic() - started}
    write(directory / (stamp + '.invocation.json'), evidence)
    return code


def jsonl(path):
    return [json.loads(line) for line in path.read_text().splitlines() if line.strip()]


def outputs(directory, output='out'):
    root = directory / output
    values = jsonl(root / 'restate-results.jsonl')
    require(values and all(all(k in v for k in ('record', 'issues', 'address', 'attempt'))
                           for v in values), 'RowOutput contract missing')
    require((root / 'issues.jsonl').exists(), 'missing issues projection')
    report = json.loads((root / 'coverage.json').read_text())
    source = json.loads((directory / 'source.json').read_text())
    require(report['total_rows'] == len(source), f'population changed: {report}')
    return values


def positive(directory, output='out', no_ai=False):
    values = outputs(directory, output)
    rows = [v['record'] for v in values]
    require([r['status'] for r in rows] == ['MATCH', 'INPUT_ERROR'], 'wrong successful outcomes')
    source = json.loads((directory / 'source.json').read_text())
    require([r['prospect']['source_fields'] for r in rows] == source, 'source JSON changed')
    with (directory / output / 'matches.csv').open(newline='') as stream:
        first = next(csv.DictReader(stream))
    require({h: first[h] for h in base.HEADERS} == source[0], 'source CSV changed')
    require(rows[0]['track_confirmed'] and rows[0]['xc_confirmed'], 'missing sport attribution')
    if no_ai:
        require(rows[0]['model_decision']['model_status'] == 'not_run_deterministic', 'no-ai provenance')
    return values


def check(args):
    evidence = []
    for name in ('no-ai', 'success', 'second-extractor', 'reviewer', 'isolation', 'postal', 'mismatch'):
        before = reset(args)
        rows = None
        if name == 'isolation':
            rows = [dict(base.PERSON, **{'Person First': 'Blocked'}), base.PERSON, base.BLANK]
        if name == 'postal':
            rows = [dict(base.PERSON, **{base.HEADERS[6]: 'not-a-postal'}), base.BLANK]
        directory = setup(args, name, rows)
        extra = ['--no-ai'] if name == 'no-ai' else []
        if name == 'second-extractor':
            control(args, {'extractor': {'candidate_status': {'xc': 503}}})
        if name == 'reviewer':
            control(args, {'reviewer': {'status': 503}})
        if name == 'isolation':
            control(args, {'search': {'blocked': True}})
        config = None
        if name == 'mismatch':
            config = directory / 'different.toml'
            config.write_text((args.root / 'config.toml').read_text() + '\n# digest mismatch\n')
        code = invoke(args, directory, extra=extra, config=config)
        after = control(args)
        if name == 'mismatch':
            require(code != 0, 'config mismatch accepted')
            require(counts(before) == counts(after), 'config mismatch had external effects')
        elif name in ('second-extractor', 'reviewer'):
            require(code != 0, 'model failure claimed success')
            failed = outputs(directory)
            require(failed[0]['record']['status'] == 'AI_ERROR' and failed[0]['issues'], 'missing durable model error')
            require(not failed[0]['record']['selected_profile_url'], 'error attributed identity')
            if name == 'second-extractor':
                require(after['extractor']['candidates'].get('tf', 0) > before['extractor']['candidates'].get('tf', 0),
                        'first candidate did not complete before failed second candidate')
            reset(args)
            require(invoke(args, directory) == 0, 'model recovery failed')
            recovered = positive(directory)
            final = control(args)
            require(final['search']['requests'] == after['search']['requests'], 'recovery repeated discovery')
            require(final['extractor']['candidates'].get('tf', 0) == after['extractor']['candidates'].get('tf', 0),
                    'recovery repeated first extraction')
            if name == 'reviewer':
                require(final['extractor']['requests'] == after['extractor']['requests'], 'review retry repeated extraction')
            require(recovered[0]['attempt'] > failed[0]['attempt'], 'retry attempt did not advance')
            require(recovered[0]['issues'], 'historical issues discarded')
        elif name == 'isolation':
            require(code != 0, 'blocked row claimed success')
            result = outputs(directory)
            require([v['record']['status'] for v in result] == ['SEARCH_ERROR', 'MATCH', 'INPUT_ERROR'],
                    'bad row blocked another row or became rejection')
            require(result[0]['issues'], 'source error lost')
        elif name == 'postal':
            result = outputs(directory)
            require(result[0]['record']['status'] == 'REVIEW', 'invalid postal not reviewed')
            require(not result[0]['record']['selected_profile_url'], 'invalid postal retained attribution')
            require(result[0]['issues'], 'address issue missing')
        else:
            require(code == 0, 'successful case failed')
            positive(directory, no_ai=name == 'no-ai')
            if name == 'no-ai':
                require(all(after[r]['requests'] == before[r]['requests'] for r in ROLES[1:]), 'no-ai called models')
            else:
                require(after['extractor']['requests'] - before['extractor']['requests'] >= 2, 'missing extraction')
                require(after['reviewer']['requests'] > before['reviewer']['requests'], 'missing review')
            for output in ('out', 'fresh'):
                require(invoke(args, directory, output, extra) == 0, 'duplicate/fresh export failed')
                positive(directory, output, no_ai=name == 'no-ai')
                require(counts(control(args)) == counts(after), 'duplicate/fresh export made external calls')
        final = control(args)
        privacy(final)
        evidence.append({'name': name, 'initial_exit': code, 'before': before, 'after': final})
        write(args.root / 'check-progress.json', evidence)
    return evidence


def restart(args):
    directory = args.root / ('restart-' + args.kind)
    state_path = directory / 'restart-state.json'
    if args.phase == 'prepare':
        before = reset(args)
        directory = setup(args, 'restart-' + args.kind)
        require(invoke(args, directory, extra=['--max', '0']) == 0, 'binding preparation failed')
        require(counts(control(args)) == counts(before), '--max 0 had external effects')
        control(args, {'extractor': {'candidate_delay': {'xc': 65}}})
        cmd = command(args, directory)
        with (directory / 'prepare.stdout.log').open('w') as out, (directory / 'prepare.stderr.log').open('w') as err:
            process = subprocess.Popen(cmd, stdin=subprocess.DEVNULL, stdout=out, stderr=err)
            try:
                deadline = time.monotonic() + 120
                while True:
                    snapshot = control(args)
                    if snapshot['extractor']['inflight'].get('xc', 0):
                        require(snapshot['extractor']['candidates'].get('tf', 0) > before['extractor']['candidates'].get('tf', 0),
                                'expected first extraction before slow second extraction')
                        break
                    require(process.poll() is None, 'CLI ended before restart boundary; inspect prepare logs')
                    require(time.monotonic() < deadline, 'restart boundary deadline exceeded')
                    time.sleep(0.2)
                state = {'kind': args.kind, 'prepared_at': time.time(), 'before': before,
                         'boundary': snapshot, 'command': cmd}
                write(state_path, state)
            finally:
                if process.poll() is None:
                    process.terminate()
                    try:
                        process.wait(timeout=5)
                    except subprocess.TimeoutExpired:
                        process.kill()
                        process.wait(timeout=10)
        return {'prepared': True, 'state': str(state_path), 'restart_now': args.kind, 'boundary': snapshot}
    state = json.loads(state_path.read_text())
    require(state['kind'] == args.kind, 'restart state kind differs')
    snapshot = control(args)
    require(time.time() - state['prepared_at'] < 60 and snapshot['extractor']['inflight'].get('xc', 0),
            'restart verification missed controlled in-flight window; use a fresh root')
    control(args, {'extractor': {'candidate_delay': {}}})
    require(invoke(args, directory, 'verified') == 0, 'restart resume failed')
    positive(directory, 'verified')
    final = control(args)
    require(final['search']['requests'] == state['boundary']['search']['requests'], 'restart repeated discovery')
    require(final['extractor']['candidates'].get('tf', 0) == state['boundary']['extractor']['candidates'].get('tf', 0),
            'restart repeated completed first extraction')
    privacy(final)
    evidence = {'verified': True, 'kind': args.kind, 'state': state, 'after': final,
                'note': 'External supervisor must retain kill/restart logs proving the requested restart.'}
    write(directory / 'verification.json', evidence)
    return evidence


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest='command', required=True)
    fixtures = sub.add_parser('serve-fixtures')
    fixtures.add_argument('--root', type=Path, required=True)
    fixtures.add_argument('--port', type=int, default=19180)
    for name in ('check', 'restart-case'):
        child = sub.add_parser(name)
        child.add_argument('--root', type=Path, required=True)
        child.add_argument('--binary', type=Path, required=True)
        child.add_argument('--restate-url', required=True)
        if name == 'restart-case':
            child.add_argument('--phase', choices=('prepare', 'verify'), required=True)
            child.add_argument('--kind', choices=('worker', 'server'), required=True)
    args = parser.parse_args()
    args.root = args.root.resolve()
    if args.command == 'serve-fixtures':
        serve(args)
        return 0
    try:
        evidence = check(args) if args.command == 'check' else restart(args)
        summary = {'passed': True, 'command': args.command, 'root': str(args.root), 'evidence': evidence}
        code = 0
    except Exception as error:
        summary = {'passed': False, 'command': args.command, 'root': str(args.root), 'error': str(error)}
        code = 1
    write(args.root / (f'summary-{args.command}-{time.time_ns()}.json'), summary)
    print(json.dumps(summary), flush=True)
    return code


if __name__ == '__main__':
    raise SystemExit(main())
