use super::{export, generation, manifest, pipeline::Corpus, Report, SchoolAddressArgs};
use anyhow::Result;
use census_domain::school_directory::{Baseline, ChangeSet, ScheduleLedger};
use std::collections::BTreeMap;
use std::path::Path;

pub(super) fn outputs(
    args: &SchoolAddressArgs,
    corpus: &Corpus,
    changes: Option<&ChangeSet>,
    ledger: &ScheduleLedger,
    mut report: Report,
) -> Result<Vec<String>> {
    let mut files = BTreeMap::new();
    files.insert(
        "school_directory.json".to_string(),
        export::json(&corpus.entries)?,
    );
    files.insert(
        "school_directory.csv".to_string(),
        export::entries_csv(&corpus.entries)?,
    );
    files.insert(
        "baseline.json".to_string(),
        export::json(&Baseline::new(corpus.entries.clone()))?,
    );
    files.insert("update_ledger.json".to_string(), export::json(ledger)?);
    if let Some(changes) = changes {
        files.insert("changes.json".to_string(), export::json(changes)?);
    }
    let inputs = manifest::Inputs {
        lanes: report.lanes.clone(),
        baseline: input_digest(args.baseline.as_deref())?,
        ledger: input_digest(args.ledger.as_deref())?,
    };
    let manifest = manifest::Manifest::new(inputs, report.now.clone(), &files)?;
    report
        .manifest_digest
        .clone_from(&manifest.generation_digest);
    let mut outputs: Vec<String> = files
        .keys()
        .map(|name| args.out.join("current").join(name).display().to_string())
        .collect();
    outputs.push(
        args.out
            .join("current/pipeline_report.json")
            .display()
            .to_string(),
    );
    outputs.push(args.out.join("current/manifest.json").display().to_string());
    report.outputs.clone_from(&outputs);
    files.insert("pipeline_report.json".to_string(), export::json(&report)?);
    generation::publish(&args.out, files, &manifest)?;
    Ok(outputs)
}

fn input_digest(path: Option<&Path>) -> Result<Option<String>> {
    match path {
        Some(path) if path.try_exists()? => Ok(Some(export::sha256_hex(
            &manifest::verified_artifact(path)?,
        ))),
        _ => Ok(None),
    }
}
