#!/usr/bin/env python3
"""Counted inventory of live athletic.net /Search.aspx/runSearch response bodies (G1 slice).

Read-only. Every number printed here is measured over the verbatim bytes the pipeline
retained, not over a re-fetch.

Inputs (all optional, repeatable):
  --raw DIR ...       files, each the site envelope the pipeline stored as a response body:
                      {"d": {"results": "<tr>...", "count": N, "pager": "..."}}
  --evidence DIR ...  files, each a query-evidence record; every page entry names the response
                      digest plus the query string / sport / stage the page was fetched for
  --parsed DIR ...    files, each the parser's own SearchPage record for one response digest
                      (candidate and issue lists, with the row locator the parser assigned)
  --samples N         verbatim samples to print per class (default 3)

Model boundaries (mirrored deliberately, quoted from the tree):
  * row   -- src/search/parser/stream/capture.rs: selector "tr"
  * link  -- src/search/parser/stream/capture.rs: selector "a[href]"  (athlete_link() by substring)
  * URL   -- src/domain/identity/validation.rs: profile_path/validate_profile_id/profile_suffix
  * row verdict -- src/search/parser/stream/row.rs:82-110 (issue wins, else identity+selected)
  * page verdict -- src/search/progress.rs:130-190 (has_issues is page-fatal)

Usage:
  inventory-search-pages.py --raw <dir> --evidence <dir> --parsed <dir> [--samples 3]
"""
from __future__ import annotations

import argparse
import collections
import glob
import hashlib
import json
import os
import re
import sys

TR = re.compile(r"<tr[ >]", re.I)
HREF = re.compile(r"""href=["']([^"']*)["']""", re.I)
ATHLETE = re.compile(r"/athlete/", re.I)
# /athlete/<digits>/<sport>[/all] with an optional single trailing slash -> ProfileUrl::parse Ok,
# and profile_matches_sport() true for that sport (markup.rs:80-90).
SPORTED = re.compile(r"^/athlete/(\d+)/(track-and-field|cross-country)(/all)?/?$", re.I)
# /athlete/<digits>[/] -> ProfileUrl::parse Ok with empty suffix; never sport-matching.
SPORTLESS = re.compile(r"^/athlete/(\d+)/?$", re.I)
# /athlete//<anything> -> validate_profile_id() rejects the empty id.
EMPTY_ID = re.compile(r"^/athlete//", re.I)
ABS_PREFIXES = (
    "https://www.athletic.net",
    "https://athletic.net",
    "http://www.athletic.net",
    "http://athletic.net",
)
NAME = re.compile(r"""href=["']/athlete/\d+/(?:track-and-field|cross-country)(?:/all)?/?["'][^>]*>([^<]*)""", re.I)

INTERSTITIAL_MARKERS = (
    "just a moment",
    "attention required",
    "cf-challenge",
    "challenge-platform",
    "cf-ray",
    "captcha",
    "turnstile",
    "hcaptcha",
    "recaptcha",
    "pardon our interruption",
    "access denied",
    "request blocked",
    "enable javascript and cookies",
    "verify you are human",
    "unusual traffic",
)

SPORT_TOKEN = {"track-and-field": "tf", "cross-country": "xc"}


def path_of(href: str) -> str:
    """Site-origin-absolute or rooted href -> path (no scheme/host/query/fragment)."""
    value = href
    for prefix in ABS_PREFIXES:
        if value.lower().startswith(prefix):
            value = value[len(prefix):]
            break
    value = value.split("?", 1)[0].split("#", 1)[0]
    return value


def link_class(href: str):
    """(class, sport) for one href, mirroring ProfileUrl::parse + profile_matches_sport."""
    if not ATHLETE.search(href):
        return ("not_athlete", None)
    path = path_of(href)
    m = SPORTED.match(path)
    if m:
        return ("sported", SPORT_TOKEN[m.group(2).lower()])
    if SPORTLESS.match(path):
        return ("sportless", None)
    if EMPTY_ID.match(path):
        return ("empty_id", None)
    return ("other_noncanonical", None)


def rows_of(results: str):
    """Row slices over the verbatim results HTML, boundary = <tr ...> (selector "tr")."""
    parts = TR.split(results)
    return parts[1:]


def analyse_row(row_html: str, sport: str | None):
    hrefs = HREF.findall(row_html)
    athlete = [(h, *link_class(h)) for h in hrefs if ATHLETE.search(h)]
    noncanonical = [h for h, cls, _ in athlete if cls in ("empty_id", "other_noncanonical")]
    sported = [s for _, cls, s in athlete if cls == "sported"]
    identity = any(cls in ("sported", "sportless") for _, cls, _ in athlete)
    selected = sport is not None and sport in sported
    issues = []
    if noncanonical:
        issues.append("athlete URL is not canonical")
    if identity and not selected:
        issues.append("result row belongs to another or unspecified sport")
    names = [n.strip() for n in NAME.findall(row_html)]
    if identity and selected and names and not names[0]:
        issues.append("athlete display name is absent or exceeds bound")
    return {
        "hrefs": hrefs,
        "athlete_hrefs": [h for h, _, _ in athlete],
        "noncanonical": noncanonical,
        "sported": sported,
        "identity": identity,
        "selected": selected,
        "issues": issues,
        "candidate": identity and selected and not issues,
    }


def read_dir(pattern_dirs, suffix=".body"):
    out = {}
    for directory in pattern_dirs:
        for path in sorted(glob.glob(os.path.join(directory, "*.body"))):
            out[os.path.basename(path)[: -len(suffix)]] = path
    return out


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--raw", action="append", default=[])
    ap.add_argument("--evidence", action="append", default=[])
    ap.add_argument("--parsed", action="append", default=[])
    ap.add_argument("--samples", type=int, default=3)
    ap.add_argument("--label", default="set")
    args = ap.parse_args()

    raw_files = read_dir(args.raw)
    evidence_files = read_dir(args.evidence)
    parsed_files = read_dir(args.parsed)
    records = []

    print(f"== {args.label} ==")
    print(f"raw bodies: {len(raw_files)}  evidence records: {len(evidence_files)}  parsed records: {len(parsed_files)}")

    # ---- evidence join: response digest -> (query, sport, stage, page index, advertised bytes)
    join = collections.defaultdict(list)
    failures = collections.Counter()
    record_complete = collections.Counter()
    for name, path in evidence_files.items():
        try:
            rec = json.loads(open(path, "rb").read())
        except Exception as exc:  # noqa: BLE001 - report the byte-level reason
            print(f"  EVIDENCE PARSE FAIL {name[:12]}: {exc!r}")
            continue
        q = rec.get("query", {})
        records.append((name, rec))
        record_complete[str(rec.get("complete"))] += 1
        for failure in rec.get("failures", []):
            if isinstance(failure, dict):
                key = f"{failure.get('code')}: {failure.get('message')}"
                evidence = failure.get("evidence") or []
                status = ",".join(str(e.get("http_status")) for e in evidence) or "-"
                failures[f"{key}  [http={status}]"] += 1
            else:
                failures[str(failure)] += 1
        for index, page in enumerate(rec.get("pages", [])):
            response = page.get("response", {})
            join[response.get("digest")].append(
                {
                    "record": name,
                    "query": q.get("query"),
                    "sport": q.get("sport"),
                    "stage": q.get("stage"),
                    "index": index,
                    "bytes": response.get("bytes"),
                    "http_status": response.get("http_status"),
                    "media_type": response.get("media_type"),
                    "source_url": response.get("source_url"),
                    "elapsed_ms": response.get("elapsed_ms"),
                    "fetched_at_unix_ms": response.get("fetched_at_unix_ms"),
                    "parsed": page.get("parsed"),
                }
            )

    print(f"evidence: complete={dict(record_complete)}")
    print(f"evidence failures ({sum(failures.values())} total):")
    for message, count in failures.most_common():
        print(f"  {count:5}  {message}")
    if not failures:
        print("  (none)")

    # ---- parser's own verdicts, from the parsed SearchPage records
    parser_pages = {}
    for name, path in parsed_files.items():
        try:
            rec = json.loads(open(path, "rb").read())
        except Exception as exc:  # noqa: BLE001
            print(f"  PARSED PARSE FAIL {name[:12]}: {exc!r}")
            continue
        parser_pages[name] = rec

    # ---- raw bodies
    link_totals = collections.Counter()
    verdict_totals = collections.Counter()
    class_samples = collections.defaultdict(list)
    count_vs_rows = collections.Counter()
    interstitials = collections.Counter()
    per_page = []
    filter_totals = collections.Counter()
    digest_mismatch = []
    byte_mismatch = []
    disagreements = []

    for digest, path in sorted(raw_files.items()):
        blob = open(path, "rb").read()
        sha = hashlib.sha256(blob).hexdigest()
        if sha != digest:
            digest_mismatch.append((digest, sha))
        entries = join.get(digest, [])
        for entry in entries:
            if entry["bytes"] is not None and int(entry["bytes"]) != len(blob):
                byte_mismatch.append((digest, entry["bytes"], len(blob)))
        sports = {entry["sport"] for entry in entries}
        sport = None
        if len(sports) == 1:
            token = next(iter(sports))
            sport = {"track_field": "tf", "cross_country": "xc"}.get(token, token)
        filter_totals[sport or "unknown/ambiguous"] += 1

        text = blob.decode("utf-8", "replace")
        lower = text.lower()
        for marker in INTERSTITIAL_MARKERS:
            if marker in lower:
                interstitials[marker] += 1

        try:
            envelope = json.loads(text)
        except Exception as exc:  # noqa: BLE001
            verdict_totals["envelope_unparseable"] += 1
            print(f"  ENVELOPE PARSE FAIL {digest[:12]}: {exc!r}")
            continue
        d = envelope.get("d") if isinstance(envelope, dict) else None
        if not isinstance(d, dict):
            verdict_totals["envelope_missing_d"] += 1
            continue
        results = d.get("results") or ""
        count = d.get("count")
        rows = rows_of(results)
        row_analyses = [analyse_row(row, sport) for row in rows]
        row_issues = [issue for a in row_analyses for issue in a["issues"]]
        candidates = [a for a in row_analyses if a["candidate"]]
        noncanonical_total = sum(len(a["noncanonical"]) for a in row_analyses)
        sported_total = sum(len(a["sported"]) for a in row_analyses)
        tf_hrefs = sum(1 for a in row_analyses for s in a["sported"] if s == "tf")
        xc_hrefs = sum(1 for a in row_analyses for s in a["sported"] if s == "xc")

        link_totals["rows_total"] += len(rows)
        link_totals["rows_with_athlete_href"] += sum(1 for a in row_analyses if a["athlete_hrefs"])
        link_totals["rows_without_athlete_href"] += sum(1 for a in row_analyses if not a["athlete_hrefs"])
        link_totals["athlete_hrefs_total"] += sum(len(a["athlete_hrefs"]) for a in row_analyses)
        link_totals["athlete_hrefs_sported_tf"] += tf_hrefs
        link_totals["athlete_hrefs_sported_xc"] += xc_hrefs
        link_totals["athlete_hrefs_noncanonical"] += noncanonical_total
        link_totals["rows_multi_athlete_href"] += sum(1 for a in row_analyses if len(a["athlete_hrefs"]) > 1)
        link_totals["rows_candidate_predicted"] += len(candidates)
        for issue in row_issues:
            verdict_totals[f"row_issue:{issue}"] += 1

        if count is None:
            count_vs_rows["count_absent"] += 1
        elif count == len(rows):
            count_vs_rows["count_eq_rows"] += 1
        elif count > len(rows):
            count_vs_rows["count_gt_rows"] += 1
        else:
            count_vs_rows["count_lt_rows"] += 1

        # pager: does the page advertise a next offset?
        pager = d.get("pager") or ""
        has_next = "data-start" in pager
        pager_pages = len(re.findall(r"<li", pager))

        parsed_digests = {e["parsed"] for e in entries if e.get("parsed")}
        parser_verdicts = set()
        parser_issues = collections.Counter()
        parser_candidates = None
        parser_count = None
        parser_next = None
        for parsed_digest in sorted(parsed_digests):
            parser = parser_pages.get(parsed_digest)
            if parser is None:
                continue
            issues_here = collections.Counter(str(i.get("message")) for i in parser.get("issues") or [])
            parser_issues.update(issues_here)
            parser_candidates = len(parser.get("candidates") or [])
            parser_count = parser.get("count")
            parser_next = parser.get("next_offset")
            if issues_here:
                parser_verdicts.add("row_issues")
            elif parser_count is not None and parser_count > parser_candidates and parser_next is None:
                parser_verdicts.add("short_page")
            elif parser_count == parser_candidates and parser_next is None:
                parser_verdicts.add("clean_full")
            else:
                parser_verdicts.add("paginated")
        if len(parser_verdicts) > 1:
            verdict_totals["ambiguous_parser_verdict"] += 1
        for verdict in parser_verdicts:
            verdict_totals[f"parser_page:{verdict}"] += 1

        # recount vs the parser's own numbers over the same bytes. Only meaningful when the page
        # was parsed for exactly one (query, sport): a shared body parsed twice has two verdicts.
        if parser_candidates is not None and sport is not None and len(parsed_digests) == 1:
            if parser_candidates != len(candidates):
                verdict_totals["candidate_count_disagrees_with_parser"] += 1
                disagreements.append((digest, len(candidates), parser_candidates, False))
            mine = collections.Counter(row_issues)
            modelled = (
                "athlete URL is not canonical",
                "result row belongs to another or unspecified sport",
            )
            for message in modelled:
                if mine.get(message, 0) != parser_issues.get(message, 0):
                    verdict_totals[f"issue_count_disagrees_with_parser:{message}"] += 1
                    disagreements.append((digest, mine.get(message, 0), parser_issues.get(message, 0), True))
            for message, tally in parser_issues.items():
                if message not in modelled:
                    verdict_totals[f"parser_issue_unmodelled:{message}"] += tally

        for a in row_analyses:
            for href in a["noncanonical"]:
                if len(class_samples["noncanonical"]) < args.samples:
                    class_samples["noncanonical"].append((digest[:12], href))
        if count is not None and count > len(rows):
            text_s = rows[0] if rows else ""
            if len(class_samples["count_gt_rows"]) < args.samples:
                class_samples["count_gt_rows"].append((digest[:12], sport, count, len(rows), pager[:200]))

        per_page.append(
            {
                "digest": digest,
                "sport": sport,
                "entries": entries,
                "queries": sorted({(e["query"], e["index"]) for e in entries}),
                "count": count,
                "rows": len(rows),
                "tf_hrefs": tf_hrefs,
                "xc_hrefs": xc_hrefs,
                "noncanonical": noncanonical_total,
                "row_issues": collections.Counter(row_issues),
                "candidates": len(candidates),
                "has_next": has_next,
                "pager_pages": pager_pages,
                "parser_candidates": parser_candidates,
                "parser_issues": parser_issues,
                "parser_verdicts": sorted(parser_verdicts),
                "parser_count": parser_count,
                "parser_next": parser_next,
            }
        )

    print("link inventory:")
    for key in (
        "rows_total",
        "rows_with_athlete_href",
        "rows_without_athlete_href",
        "athlete_hrefs_total",
        "athlete_hrefs_sported_tf",
        "athlete_hrefs_sported_xc",
        "athlete_hrefs_noncanonical",
        "rows_multi_athlete_href",
        "rows_candidate_predicted",
    ):
        print(f"  {link_totals[key]:7}  {key}")
    print("count vs rows:", dict(count_vs_rows))
    reject_issues = [p for p in per_page if p["row_issues"]]
    reject_short = [
        p for p in per_page if p["count"] is not None and p["count"] > p["rows"] and not p["has_next"]
    ]
    reject_count_lt = [p for p in per_page if p["count"] is not None and p["count"] < p["rows"]]
    clean = [
        p
        for p in per_page
        if not p["row_issues"] and p["count"] is not None and p["count"] == p["rows"] and not p["has_next"]
    ]
    with_candidates = [p for p in per_page if p["candidates"] > 0]
    salvageable = [p for p in with_candidates if p["row_issues"]]
    print("page verdicts:")
    print("  recount (bytes only):")
    print(
        f"    pages={len(per_page)} row_issue_pages={len(reject_issues)} "
        f"short_page={len(reject_short)} count_lt_rows={len(reject_count_lt)} clean_full={len(clean)}"
    )
    print(
        f"    pages_carrying_candidate_rows={len(with_candidates)} "
        f"of_which_row_issue_pages={len(salvageable)} "
        f"candidate_rows_lost_on_those_pages={sum(p['candidates'] for p in salvageable)}"
    )
    parser_issue_pages = [p for p in per_page if p["parser_issues"]]
    parser_short_pages = [p for p in per_page if "short_page" in p["parser_verdicts"]]
    parser_clean_pages = [p for p in per_page if p["parser_verdicts"] == ["clean_full"]]
    parser_cand_pages = [p for p in per_page if (p["parser_candidates"] or 0) > 0]
    parser_salvageable = [p for p in parser_cand_pages if p["parser_issues"]]
    print("  parser (its own records for these same bytes):")
    print(
        f"    pages={len(per_page)} row_issue_pages={len(parser_issue_pages)} "
        f"short_page={len(parser_short_pages)} clean_full={len(parser_clean_pages)} "
        f"no_parsed_record={sum(1 for p in per_page if not p['parser_verdicts'])}"
    )
    print(
        f"    pages_with_candidates={len(parser_cand_pages)} "
        f"of_which_row_issue_pages={len(parser_salvageable)} "
        f"candidate_rows_on_those_pages={sum(p['parser_candidates'] for p in parser_salvageable)}"
    )
    only_noncanonical = [
        p for p in parser_issue_pages if set(p["parser_issues"]) == {"athlete URL is not canonical"}
    ]
    only_sport = [
        p
        for p in parser_issue_pages
        if set(p["parser_issues"]) == {"result row belongs to another or unspecified sport"}
    ]
    both = [
        p
        for p in parser_issue_pages
        if len(set(p["parser_issues"])) > 1
        or set(p["parser_issues"]) not in [{"athlete URL is not canonical"}, {"result row belongs to another or unspecified sport"}]
    ]
    print("  issue classes on row-issue pages:")
    print(f"    only_non_canonical_url={len(only_noncanonical)}")
    print(f"    only_other_or_unspecified_sport={len(only_sport)}")
    print(f"    both_or_other={len(both)}")
    co_mingled = [p for p in per_page if p["tf_hrefs"] and p["xc_hrefs"]]
    print("  pages carrying rows of both sports in one response (tf hrefs and xc hrefs):")
    print(
        f"    co_mingled_pages={len(co_mingled)} "
        f"with_row_issues={sum(1 for p in co_mingled if p['row_issues'])} "
        f"same_sport_candidate_rows_on_them={sum(p['candidates'] for p in co_mingled)}"
    )
    last_page_short = [
        p
        for p in parser_issue_pages
        if p["parser_next"] is None and (p["parser_count"] or 0) > (p["parser_candidates"] or 0)
    ]
    with_next = [p for p in parser_issue_pages if p["parser_next"] is not None]
    print("  if other-sport rows became skips instead of issues:")
    print(
        f"    row_issue_pages_that_are_mid_pagination(next present)={len(with_next)} "
        f"row_issue_pages_that_are_last_page_and_short={len(last_page_short)}"
    )
    issue_page_digests = {p["digest"] for p in parser_issue_pages}
    short_page_digests = {p["digest"] for p in parser_short_pages}
    records_crosscheck = collections.Counter()
    failure_classes = collections.Counter()
    unexplained = []
    for name, rec in records:
        failed = not rec.get("complete")
        messages = [
            str(failure.get("message")) if isinstance(failure, dict) else str(failure)
            for failure in rec.get("failures", [])
        ]
        outcomes = []
        for page in rec.get("pages", []):
            parser = parser_pages.get(page.get("parsed"))
            if parser is None:
                outcomes.append(None)
            elif parser.get("issues"):
                outcomes.append("issues")
            else:
                outcomes.append("clean")
        has_issue = any(outcome == "issues" for outcome in outcomes)
        row_issue_failure = any("unrelated result rows" in m for m in messages)
        short_failure = any("ended before the advertised" in m for m in messages)
        records_crosscheck["complete" if not failed else "failed"] += 1
        records_crosscheck[f"  {'complete' if not failed else 'failed'}:own_parse_has_row_issues={has_issue}"] += 1
        if failed:
            records_crosscheck[f"  failed:row_issue_failure={row_issue_failure}"] += 1
            records_crosscheck[f"  failed:short_failure={short_failure}"] += 1
            exceed = any("advertised result count exceeds" in m for m in messages)
            classes = []
            if row_issue_failure:
                classes.append("row_issues")
            if short_failure:
                classes.append("short_page")
            if exceed:
                classes.append("advertised_count_exceeds")
            failure_classes["+".join(classes) if classes else "other"] += 1
        if not failed and has_issue:
            records_crosscheck["  COMPLETE_WITH_ROW_ISSUES"] += 1
            unexplained.append((name, failed, has_issue, False, messages))
        if failed and not has_issue and not row_issue_failure:
            records_crosscheck["  FAILED_WITHOUT_ROW_ISSUES"] += 1
            unexplained.append((name, failed, has_issue, short_failure, messages))
    print("  record outcomes (the pipeline's own complete/failures) vs page verdicts:")
    for key, value in sorted(records_crosscheck.items()):
        print(f"    {value:5}  {key}")
    print("  rejected-record classes (which guard killed the query):")
    for key, value in sorted(failure_classes.items(), key=lambda kv: -kv[1]):
        print(f"    {value:5}  {key}")
    for name, failed, has_issue, has_short, messages in unexplained[:4]:
        print(f"    unexplained {name[:12]} failed={failed} page_issues={has_issue} page_short={has_short} {messages[:2]}")
    print("verdict totals:")
    for key, value in sorted(verdict_totals.items()):
        print(f"  {value:7}  {key}")
    print("page filter (sport the page was fetched for):", dict(filter_totals))
    print(f"interstitial markers: {dict(interstitials) or 'none'}")
    print(f"raw-body sha256 mismatches: {len(digest_mismatch)}  evidence-bytes mismatches: {len(byte_mismatch)}")
    for digest, sha in digest_mismatch[:3]:
        print(f"  sha mismatch {digest[:12]} -> {sha[:12]}")
    for digest, claimed, actual in byte_mismatch[:3]:
        print(f"  bytes mismatch {digest[:12]} claimed={claimed} actual={actual}")

    print("samples:")
    for name, values in class_samples.items():
        for value in values:
            print(f"  [{name}] {value}")

    print("per-page (digest, filter, count, rows, tf, xc, noncanon, predicted issues, candidates, parser candidates/issues):")
    for page in per_page:
        queries = ",".join(sorted({q for q, _ in page["queries"]}))
        issues = ",".join(f"{k}:{v}" for k, v in page["row_issues"].items())
        parser_issues = ",".join(f"{k}:{v}" for k, v in page["parser_issues"].items())
        print(
            f"  {page['digest'][:12]} {str(page['sport']):3} count={page['count']!s:>5} rows={page['rows']:3} "
            f"tf={page['tf_hrefs']:3} xc={page['xc_hrefs']:3} noncanon={page['noncanonical']:2} "
            f"next={'y' if page['has_next'] else 'n'} pred_issues=[{issues}] cand={page['candidates']} "
            f"parser_cand={page['parser_candidates']} parser_issues=[{parser_issues}] q={queries[:60]}"
        )
    return 0


if __name__ == "__main__":
    sys.exit(main())
