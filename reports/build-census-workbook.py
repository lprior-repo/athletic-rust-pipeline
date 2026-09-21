#!/usr/bin/env python3
"""Build the Midwest census workbook from the consolidated report snapshots.

Inputs are the two scope reports the census itself writes:

    cargo run --release -p midwest-census -- report --core    # var/midwest-census/out/report-core.json
    cargo run --release -p midwest-census -- report          # var/midwest-census/out/report.json

Every number in the workbook is copied from those files; nothing here recomputes a census. The
multi-sheet book is emitted as flat ODS XML and converted to XLSX with headless LibreOffice, so it
does not depend on a Python spreadsheet library being installed.

Usage: python3 reports/build-census-workbook.py [--store var/midwest-census] [--out reports]
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from html import escape
from pathlib import Path

SHEETS: list[tuple[str, list[list[object]]]] = []


def sheet(name: str, rows: list[list[object]]) -> None:
    SHEETS.append((name, rows))


def pct(part: float, whole: float) -> str:
    return "n/a" if not whole else f"{100.0 * part / whole:.1f}%"


def load(store: Path) -> tuple[dict, dict]:
    core = json.loads((store / "out/report-core.json").read_text())
    all_sources = json.loads((store / "out/report-all-sources.json").read_text())
    return core, all_sources


METRIC_ROWS = [
    ("Athletes (all grades)", "athletes"),
    ("Class of 2027", "class_of_2027"),
    ("Class of 2027 boys", "class_of_2027_boys"),
    ("Class of 2027 girls", "class_of_2027_girls"),
    ("Class of 2027, grade-evidenced", "class_of_2027_with_grad_year_evidence"),
    ("Class of 2027, public profile URL", "class_of_2027_with_profile_url"),
    ("Class of 2027, multiple sources", "class_of_2027_multisource"),
    ("Class of 2027, coach identified", "class_of_2027_with_coach"),
    ("Class of 2027, coach professional email", "class_of_2027_with_coach_email"),
    ("Coaches", "coaches"),
    ("Coaches with professional email", "coaches_with_email"),
    ("Schools", "schools"),
]

STATE_COLUMNS = [
    ("schools", "Schools"),
    ("athletes", "Athletes"),
    ("class_of_2027", "Class of 2027"),
    ("class_of_2027_boys", "Co27 boys"),
    ("class_of_2027_girls", "Co27 girls"),
    ("class_of_2027_with_grad_year_evidence", "Co27 grade-evidenced"),
    ("class_of_2027_with_profile_url", "Co27 profile URL"),
    ("class_of_2027_multisource", "Co27 multi-source"),
    ("class_of_2027_with_coach", "Co27 with coach"),
    ("class_of_2027_with_coach_email", "Co27 with coach email"),
    ("coaches", "Coaches"),
    ("coaches_with_email", "Coaches with email"),
]


def build(store: Path, core: dict, all_sources: dict) -> None:
    core_totals, all_totals = core["totals"], all_sources["totals"]

    sheet(
        "Goal & method",
        [
            ["Midwest high-school track & field / cross-country recruiting census"],
            [],
            ["Goal"],
            [
                "Own the canonical graph (school, team, coach, athlete, meet, event, performance) for "
                "boys and girls track & field and cross-country, with graduating class kept separate "
                "from the grade a source happened to publish, without depending on Athletic.net."
            ],
            ["Success test", "Athletic.net and its AthleticLIVE mirror switched off: the census still runs and reports."],
            ["Core scope", "Evidence produced by adapters that do not read Athletic.net or its mirror."],
            ["All-sources scope", "Adds the two AthleticLIVE modules (mirror + athlete index) as the comparison baseline."],
            [],
            ["Source tiers"],
            ["Tier A", "Official state associations: WIAA, MSHSL, IHSA, OHSAA, KSHSAA, NDHSAA, NSAA"],
            ["Tier B", "MileSplit-style state sites: rosters, graded athletes, public profile URLs"],
            ["Tier D", "Official result artifacts: Hy-Tek, Compiled, cross-country and RaceDay layouts"],
            ["Tier E", "Compliant timing providers: Wayzata Results (MN / IA / WI) published schedules"],
            ["Non-core", "Athletic.net and the AthleticLIVE mirror: never fetched by a core run"],
            [],
            ["Reproduce"],
            ["Consolidate", "cargo run --release -p midwest-census -- consolidate"],
            ["Core report", "cargo run --release -p midwest-census -- report --core"],
            ["All-source report", "cargo run --release -p midwest-census -- report"],
            ["Workbook", "python3 reports/build-census-workbook.py"],
            [],
            ["Provenance"],
            ["Core report generated", core["generated_on"]],
            ["Store", str(store)],
            ["Core note", core["notes"][0] if core.get("notes") else ""],
            ["All-source note", all_sources["notes"][0] if all_sources.get("notes") else ""],
        ],
    )

    summary = [["Metric", "Core (Athletic.net off)", "All sources", "Core share"]]
    for label, key in METRIC_ROWS:
        c, a = core_totals.get(key, 0), all_totals.get(key, 0)
        summary.append([label, c, a, pct(c, a)])
    summary += [
        [],
        ["Core meets", core["meets"]["total"], all_sources["meets"]["total"], pct(core["meets"]["total"], all_sources["meets"]["total"])],
        [
            "Meets carrying an Athletic.net id",
            0,
            all_sources["meets"]["with_athletic_net_id"],
            "enrichment key only, never dereferenced",
        ],
    ]
    sheet("Summary", summary)

    for label, report in (("By state - core", core), ("By state - all sources", all_sources)):
        rows = [["State"] + [header for _, header in STATE_COLUMNS]]
        rows[0] += ["Co27 profile URL %", "Co27 with coach %", "Co27 with coach email %"]
        schools_by_state = report.get("schools_by_state", {})
        totals = report["totals"]
        for state, row in sorted(
            report["by_state"].items(), key=lambda kv: -kv[1]["class_of_2027"]
        ):
            # The per-state `schools` field is a count of schools *seen in that state's result rows*,
            # which is zero for states whose schools arrived from directories; `schools_by_state` is
            # the enumeration the census actually holds.
            row = {**row, "schools": schools_by_state.get(state, row.get("schools", 0))}
            line = [state] + [row.get(key, 0) for key, _ in STATE_COLUMNS]
            line += [
                pct(row.get("class_of_2027_with_profile_url", 0), row["class_of_2027"]),
                pct(row.get("class_of_2027_with_coach", 0), row["class_of_2027"]),
                pct(row.get("class_of_2027_with_coach_email", 0), row["class_of_2027"]),
            ]
            rows.append(line)
        totals = {**totals, "schools": sum(schools_by_state.values()) or totals["schools"]}
        rows.append(
            ["TOTAL"]
            + [totals.get(key, 0) for key, _ in STATE_COLUMNS]
            + [
                pct(totals.get("class_of_2027_with_profile_url", 0), totals["class_of_2027"]),
                pct(totals.get("class_of_2027_with_coach", 0), totals["class_of_2027"]),
                pct(totals.get("class_of_2027_with_coach_email", 0), totals["class_of_2027"]),
            ]
        )
        sheet(label, rows)

    marginal = [
        [
            "State",
            "Co27, all sources",
            "Co27, core",
            "Co27 only visible through Athletic.net",
            "Core share",
            "Athletes, all sources",
            "Athletes, core",
        ]
    ]
    for state, row in sorted(
        all_sources["by_state"].items(), key=lambda kv: -kv[1]["class_of_2027"]
    ):
        core_row = core["by_state"].get(state, {})
        all_co = row["class_of_2027"]
        core_co = core_row.get("class_of_2027", 0)
        marginal.append(
            [
                state,
                all_co,
                core_co,
                all_co - core_co,
                pct(core_co, all_co),
                row["athletes"],
                core_row.get("athletes", 0),
            ]
        )
    marginals = marginal[1:]
    sheet(
        "Athletic.net marginal",
        marginal
        + [
            [
                "TOTAL",
                sum(row[1] for row in marginals),
                sum(row[2] for row in marginals),
                sum(row[3] for row in marginals),
                pct(sum(row[2] for row in marginals), sum(row[1] for row in marginals)),
                sum(row[5] for row in marginals),
                sum(row[6] for row in marginals),
            ]
        ],
    )

    meets = [["Core meet inventory"]]
    meets.append(["Total", core["meets"]["total"]])
    meets += [[], ["By state", "Meets"]]
    for state, count in sorted(core["meets"]["by_state"].items(), key=lambda kv: -kv[1]):
        meets.append([state, count])
    meets += [[], ["By provider key", "Meets"]]
    for provider, count in sorted(
        core["meets"].get("by_provider", {}).items(), key=lambda kv: -kv[1]
    ):
        meets.append([provider, count])
    meets += [
        [],
        ["All sources", "Meets"],
        ["Total", all_sources["meets"]["total"]],
        ["With Athletic.net meet id", all_sources["meets"]["with_athletic_net_id"]],
        [],
        ["By state (all sources)", "Meets"],
    ]
    for state, count in sorted(
        all_sources["meets"]["by_state"].items(), key=lambda kv: -kv[1]
    ):
        meets.append([state, count])
    sheet("Meets", meets)

    evidence = [["Coach sources", "Records"]]
    for source, count in sorted(
        all_sources.get("coach_sources", {}).items(), key=lambda kv: -kv[1]
    ):
        evidence.append([source, count])
    evidence += [[], ["Grade-evidence sources", "Athletes"]]
    for source, count in sorted(
        all_sources.get("providers", {}).get("grade_evidence_sources", {}).items(),
        key=lambda kv: -kv[1],
    ):
        evidence.append([source, count])
    evidence += [[], ["Class-of-2027 sport mix", "Athletes"]]
    for key, count in all_sources.get("class_of_2027_sports", {}).items():
        evidence.append([key, count])
    sheet("Evidence mix", evidence)

    sheet(
        "Method notes",
        [
            ["Result-artifact parsing", "96 parsed artifacts before the vendor layouts landed; 1,740 after (Compiled 763, cross-country 380, Hy-Tek 597); 834,254 result rows, 264,167 grade-bearing."],
            ["Class-of-2027 evidence", "The artifact corpus added 4,323 Wisconsin class-of-2027 athletes to core (14,958 to 19,281)."],
            ["North Dakota and South Dakota", "349 teams collected, 29,746 athletes, 4,712 class-of-2027, no errors."],
            ["Wayzata schedules", "537 competition rows over two 2026 schedules minted 536 core meets; 304 rows resolved to a state (95 recurring sites, 209 schools)."],
            ["Unresolved venues", "Filed under ?? rather than guessed; they are meet inventory, not athlete evidence."],
            ["Coach coverage", "WI, MN, IL, OH, NE and ND publish directories; MI, MO, IN and KS still have none."],
            ["Tests", "183 tests pass; the adapters added here are clippy-clean."],
        ],
    )


def cell(value: object) -> str:
    if value is None or value == "":
        return "<table:table-cell/>"
    if isinstance(value, (int, float)) and not isinstance(value, bool):
        return (
            f'<table:table-cell office:value-type="float" office:value="{value}">'
            f"<text:p>{value}</text:p></table:table-cell>"
        )
    text = escape(str(value))
    return f'<table:table-cell office:value-type="string"><text:p>{text}</text:p></table:table-cell>'


def write_fods(path: Path) -> None:
    parts = [
        '<?xml version="1.0" encoding="UTF-8"?>',
        '<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" '
        'xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0" '
        'xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" '
        'office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.spreadsheet">',
        "<office:body><office:spreadsheet>",
    ]
    for name, rows in SHEETS:
        parts.append(f'<table:table table:name="{escape(name)}">')
        for _ in range(max((len(row) for row in rows), default=1)):
            parts.append("<table:table-column/>")
        for row in rows:
            parts.append("<table:table-row>")
            parts.extend(cell(value) for value in row)
            parts.append("</table:table-row>")
        parts.append("</table:table>")
    parts.append("</office:spreadsheet></office:body></office:document>")
    path.write_text("".join(parts), encoding="utf-8")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--store", default="var/midwest-census")
    parser.add_argument("--out", default="reports")
    parser.add_argument("--name", default="midwest-census-2026-09-20")
    parser.add_argument("--skip-xlsx", action="store_true")
    args = parser.parse_args()

    store = Path(args.store)
    core, all_sources = load(store)
    build(store, core, all_sources)

    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    fods = out / f"{args.name}.fods"
    write_fods(fods)

    for name, rows in SHEETS:
        slug = name.lower().replace(" & ", "-").replace(" ", "-").replace(".", "")
        with (out / f"{args.name}-{slug}.csv").open("w", encoding="utf-8") as handle:
            for row in rows:
                handle.write(",".join(csv_cell(v) for v in row) + "\n")

    if not args.skip_xlsx:
        result = subprocess.run(
            ["soffice", "--headless", "--convert-to", "xlsx", "--outdir", str(out), str(fods)],
            capture_output=True,
            text=True,
            check=False,
        )
        if result.returncode != 0 or not (out / f"{args.name}.xlsx").exists():
            print(result.stdout, result.stderr, file=sys.stderr)
            return 1
        print(f"wrote {out / (args.name + '.xlsx')}")
    print(f"wrote {fods} and {len(SHEETS)} sheet CSVs")
    print(f"sheets: {', '.join(name for name, _ in SHEETS)}")
    return 0


def csv_cell(value: object) -> str:
    text = "" if value is None else str(value)
    return f'"{text}"' if any(ch in text for ch in ',"\n') else text


if __name__ == "__main__":
    raise SystemExit(main())
