use super::config::{self, Manifest};
use anyhow::{ensure, Context, Result};
use census_domain::{model::SchoolYear, UsJurisdiction};
use census_report::{report::Scope, workbook};
use census_store::{Store, Table};
use std::collections::BTreeMap;
use std::path::Path;
use std::time::Instant;

const FAMILIES: [&str; 8] = [
    "athletes.csv",
    "best_marks.csv",
    "schools.csv",
    "coach_contacts.csv",
    "comparison_classes.csv",
    "provenance.csv",
    "coverage.csv",
    "identity_aliases.csv",
];

pub fn publish(root: &Path, out: &Path) -> Result<()> {
    let started = Instant::now();
    let manifest = Manifest::read(root)?;
    config::fresh_output(root, out)?;
    let store = Store::open_existing(root.join("store"))?;
    ensure!(
        store.snapshot().tables_digest(&Table::ALL)? == manifest.tables_sha256,
        "fixture store digest mismatch"
    );
    ensure!(store.walk_table(Table::Athletes)?.rows == manifest.counts.stored_athletes);
    ensure!(store.walk_table(Table::Performances)?.rows == manifest.counts.performances);
    let options = workbook::Options {
        grad_year: Some(2027),
        out: Some(out.join("bundle")),
        limit: None,
        scope: Scope::AllSources,
        jurisdictions: vec![UsJurisdiction::Michigan],
        school_year: SchoolYear::new(2025).context("fixture season")?,
    };
    let written = workbook::build(&store, &options)?;
    let built_seconds = started.elapsed().as_secs_f64();
    let verified = workbook::publication::verify_published(&written)?;
    ensure!(
        verified.workbook.mapped_athletes == manifest.counts.selected_athletes,
        "verified selected population changed"
    );
    let counts = readback(written.parent().context("bundle generation")?, &manifest)?;
    let result = serde_json::json!({
        "fixture_version": manifest.version, "fixture_sha256": manifest.fixture_sha256,
        "tables_sha256": manifest.tables_sha256, "synthetic_only": true,
        "measurement_scope": "store open/integrity, canonical workbook build, independent canonical verifier, streaming CSV checks",
        "generation_sha256": verified.generation_digest, "normalized_rows": counts,
        "mapped_athletes": verified.workbook.mapped_athletes, "workbook_rows": verified.workbook.rows,
        "build_seconds": built_seconds, "whole_child_seconds": started.elapsed().as_secs_f64(),
        "workbook": written, "census_acceptance": false,
    });
    config::write_new(
        &out.join("publication-result.json"),
        &serde_json::to_vec_pretty(&result)?,
    )?;
    println!("{}", serde_json::to_string(&result)?);
    Ok(())
}

fn readback(directory: &Path, manifest: &Manifest) -> Result<BTreeMap<String, u64>> {
    let mut counts = BTreeMap::new();
    for family in FAMILIES {
        let mut reader = csv::Reader::from_path(directory.join(family))?;
        let header = reader.headers()?.clone();
        let mut record = csv::StringRecord::new();
        let mut rows = 0u64;
        let mut flags = Flags::default();
        while reader.read_record(&mut record)? {
            ensure!(record.len() == header.len(), "malformed {family} row");
            rows = rows.checked_add(1).context("CSV row overflow")?;
            inspect(family, &header, &record, manifest, &mut flags)?;
        }
        validate_counts(family, rows, &flags, manifest)?;
        counts.insert(family.into(), rows);
    }
    Ok(counts)
}

#[derive(Default)]
struct Flags {
    aliases: u64,
    outside: u64,
    identity_only: u64,
    performance_facts: u64,
}

fn inspect(
    family: &str,
    header: &csv::StringRecord,
    row: &csv::StringRecord,
    manifest: &Manifest,
    flags: &mut Flags,
) -> Result<()> {
    match family {
        "athletes.csv" => {
            ensure!(field(header, row, "graduation_year")? == "2027");
            ensure!(field(header, row, "display_name")?.starts_with("Synthetic "));
            if field(header, row, "performance_availability")?
                == "no_eligible_selected_mark_in_snapshot"
            {
                flags.identity_only = flags
                    .identity_only
                    .checked_add(1)
                    .context("identity_only overflow")?;
            }
        }
        "best_marks.csv" => {
            ensure!(field(header, row, "meet_date")? == config::DAY);
            ensure!(
                field(header, row, "source_url")?.starts_with("https://example.test/synthetic/")
            );
        }
        "identity_aliases.csv" => {
            if field(header, row, "original_id")? != field(header, row, "resolved_id")? {
                flags.aliases = flags.aliases.checked_add(1).context("aliases overflow")?;
            }
            if field(header, row, "publication_membership")? == "canonical_outside_publication" {
                flags.outside = flags.outside.checked_add(1).context("outside overflow")?;
            }
        }
        "provenance.csv" => inspect_fact(header, row, manifest, flags)?,
        _ => {}
    }
    Ok(())
}

fn inspect_fact(
    header: &csv::StringRecord,
    row: &csv::StringRecord,
    manifest: &Manifest,
    flags: &mut Flags,
) -> Result<()> {
    let url = field(header, row, "source_url")?;
    ensure!(
        url == "unavailable" || url.starts_with("https://example.test/"),
        "non-synthetic provenance URL"
    );
    if field(header, row, "entity_type")? == "performance"
        && field(header, row, "source_reference")? == "synthetic_memory_fixture"
    {
        let fact: serde_json::Value = serde_json::from_str(field(header, row, "retained_fact")?)?;
        let note = fact
            .get("note")
            .and_then(serde_json::Value::as_str)
            .context("synthetic note missing")?;
        ensure!(
            note.len() == manifest.config.evidence_bytes && note.bytes().all(|byte| byte == b'x')
        );
        ensure!(field(header, row, "observed_on")? == config::DAY);
        flags.performance_facts = flags
            .performance_facts
            .checked_add(1)
            .context("performance_facts overflow")?;
    }
    Ok(())
}

fn validate_counts(family: &str, rows: u64, flags: &Flags, manifest: &Manifest) -> Result<()> {
    let counts = &manifest.counts;
    match family {
        "athletes.csv" => ensure!(
            rows == counts.selected_athletes && flags.identity_only == counts.identity_only,
            "athlete population/identity-only mismatch"
        ),
        "best_marks.csv" => ensure!(rows == counts.selected_bests, "best population mismatch"),
        "identity_aliases.csv" => ensure!(
            rows == counts.stored_athletes
                && flags.aliases == counts.aliases
                && flags.outside == counts.outside,
            "alias/scope population mismatch"
        ),
        "provenance.csv" => {
            let expected = counts
                .performances
                .checked_mul(u64::from(manifest.config.evidence_width))
                .context("evidence width multiplication overflow")?;
            ensure!(
                flags.performance_facts == expected,
                "performance evidence width mismatch"
            );
        }
        _ => {}
    }
    Ok(())
}

fn field<'a>(
    header: &csv::StringRecord,
    row: &'a csv::StringRecord,
    name: &str,
) -> Result<&'a str> {
    let index = header
        .iter()
        .position(|field| field == name)
        .with_context(|| format!("missing {name}"))?;
    row.get(index).with_context(|| format!("short {name}"))
}
