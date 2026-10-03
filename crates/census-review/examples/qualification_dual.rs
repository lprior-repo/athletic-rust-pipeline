use census_domain::model::{
    serialized_digest, CanonicalSchool, ReviewCase, ReviewPacket, ReviewState, ReviewVerdictRecord,
    SchoolId,
};
use census_review::{
    run_lanes, ModelClient, ModelOptions, ModelResponseFormat, ReviewFamily, ReviewOptions,
};
use census_store::{Store, Table};
use serde_json::Value;
use std::{io::Write, path::Path, time::Duration};

type Result<T, E = Box<dyn std::error::Error>> = std::result::Result<T, E>;

fn seed(store: &Store) -> Result<SchoolId> {
    let school: CanonicalSchool = serde_json::from_value(serde_json::json!({
        "id": SchoolId::mint("sch", &["SYNTHETIC QUALIFICATION ONLY", "a6c"]),
        "name": "SYNTHETIC QUALIFICATION ONLY - Example School",
        "normalized_name": "synthetic qualification only example school",
        "co_op": false, "aliases": [], "source_identities": [], "evidence": []
    }))?;
    store.append(Table::Schools, &school)?;
    let case = ReviewCase::pending(
        ReviewFamily::SchoolJurisdiction.label(),
        school.id.as_str(),
        school.name.as_str(),
        "SYNTHETIC QUALIFICATION ONLY; determine state from supplied evidence. No source capture or source ownership is asserted; city, association, athletics website and state are unknown.",
    );
    store.replace(Table::ReviewCases, &case)?;
    Ok(school.id)
}

fn clients() -> Result<Vec<ModelClient>> {
    [
        ("http://127.0.0.1:11000", ModelResponseFormat::PromptJson),
        ("http://127.0.0.1:11001", ModelResponseFormat::JsonSchema),
    ]
    .into_iter()
    .map(|(endpoint, format)| {
        let options = ModelOptions::local(endpoint, "qwen3.8-27b-uncensored")?
            .with_timeout(Duration::from_secs(90))
            .with_max_tokens(512)
            .with_response_format(format);
        Ok(ModelClient::new(options)?)
    })
    .collect()
}

fn inspect(store: &Store, school: &SchoolId, root: &Path) -> Result<()> {
    let rows = store.scan::<ReviewVerdictRecord>(Table::IdentityVerdicts)?;
    let [row] = rows.as_slice() else {
        return Err(format!("expected one verdict, got {}", rows.len()).into());
    };
    let audit: Value = serde_json::from_str(&row.rationale)?;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(root.join("dual-audit.json"))?;
    file.write_all(&serde_json::to_vec_pretty(&audit)?)?;
    println!("record={}", serde_json::to_string(row)?);
    let packet: Option<ReviewPacket> =
        serde_json::from_value(audit.get("packet").ok_or("missing packet")?.clone())?;
    let packet_digest = serialized_digest(&packet)?;
    if row.subject_id != school.as_str()
        || row.accepted
        || row.kind != "insufficient_evidence"
        || !row.field.is_empty()
        || !row.value.is_empty()
        || row.confidence != 0
        || audit.get("evidence_digest").and_then(Value::as_str) != Some(packet_digest.as_str())
    {
        return Err("unexpected unresolved record; raw audit preserved".into());
    }
    let lanes = audit
        .get("lanes")
        .and_then(Value::as_array)
        .ok_or("missing lanes")?;
    if lanes.len() != 2 {
        return Err("expected two audited lanes".into());
    }
    for (lane, format) in lanes.iter().zip(["prompt_json", "json_schema"]) {
        println!("lane={}", serde_json::to_string(lane)?);
        validate_lane(lane, format)?;
    }
    let cases = store.scan::<ReviewCase>(Table::ReviewCases)?;
    let [case] = cases.as_slice() else {
        return Err("expected one persisted case".into());
    };
    if case.id != row.case_id || case.state != ReviewState::Retained {
        return Err("unresolved case was not retained".into());
    }
    println!(
        "case_state={:?} outcome={}",
        case.state,
        audit.get("outcome").ok_or("missing outcome")?
    );
    Ok(())
}

fn validate_lane(lane: &Value, format: &str) -> Result<()> {
    let kind = lane
        .get("batch")
        .and_then(|batch| batch.get("verdicts"))
        .and_then(Value::as_array)
        .and_then(|verdicts| verdicts.first())
        .and_then(|verdict| verdict.get("kind"))
        .and_then(Value::as_str);
    if lane.get("status").and_then(Value::as_str) != Some("answered")
        || lane.get("response_format").and_then(Value::as_str) != Some(format)
        || lane.get("model").and_then(Value::as_str) != Some("qwen3.8-27b-uncensored")
        || !lane.get("error").is_some_and(Value::is_null)
        || kind != Some("insufficient_evidence")
        || lane
            .get("request_digest")
            .and_then(Value::as_str)
            .is_none_or(|digest| digest.len() != 64)
    {
        return Err("unexpected lane response/binding; raw audit preserved".into());
    }
    Ok(())
}

fn main() -> Result<()> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let root = std::env::args_os()
                .nth(1)
                .ok_or("pass fresh evidence directory")?;
            let root = Path::new(&root);
            std::fs::create_dir(root)?;
            let store = Store::open(root.join("store"))?;
            let school = seed(&store)?;
            let clients = clients()?;
            let options = ReviewOptions {
                families: vec![ReviewFamily::SchoolJurisdiction],
                limit: 1,
                dry_run: false,
            };
            let report = run_lanes(&store, &clients, &options, "2026-10-01").await?;
            println!("{}", report.summary());
            inspect(&store, &school, root)?;
            if report.requested != 1
                || report.answered != 1
                || report.accepted != 0
                || report.failed != 0
            {
                return Err(
                    "dual endpoint qualification did not yield two validated unresolved replies"
                        .into(),
                );
            }
            let replay = run_lanes(&store, &clients, &options, "2026-10-01").await?;
            println!("replay={}", replay.summary());
            if replay.requested != 0 || store.receipt_count()? != 1 {
                return Err(
                    "unchanged audit replay reissued requests or mutated checkpoint".into(),
                );
            }
            Ok(())
        })
}
