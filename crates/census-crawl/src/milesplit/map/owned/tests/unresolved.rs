use super::*;
#[test]
fn same_name_school_and_cohort_never_merge_different_positive_provider_owners() -> TestResult {
    let mut source = document()?;
    source["data"][2]["firstName"] = json!("Adelyn");
    source["data"][2]["lastName"] = json!("Spann");
    source["data"][2]["teamId"] = json!("38332");
    source["data"][2]["gradYear"] = json!("2027");
    let (accumulated, _) = project(source, &[school("Shared school", "38332")], metadata()?)?;
    let owners: std::collections::BTreeMap<_, _> = accumulated
        .athletes
        .values()
        .map(|athlete| {
            check!(eq; athlete.canonical_name, "Adelyn Spann");
            Ok((
                athlete.source.as_ref().ok_or("provider owner")?.id.as_str(),
                &athlete.id,
            ))
        })
        .collect::<TestResult<_>>()?;
    check!(ne; owners["14222592"], owners["11357806"]);
    for performance in accumulated.performances.values() {
        let owner = performance.source_athlete.as_ref().ok_or("source owner")?;
        check!(eq; &performance.athlete, owners[owner.id.as_str()]);
    }
    Ok(())
}

#[test]
fn missing_ambiguous_noncanonical_and_wrong_namespace_team_ids_remain_unresolved() -> TestResult {
    let mut wrong_namespace = school("Adelyn Spann", "38332");
    wrong_namespace.source_identities[0].namespace = SourceNamespace::MilesplitTeam;
    for (schools, reason) in [
        (
            vec![school("Charles", "4912")],
            "missing exact positive MilesplitSchool provider teamID",
        ),
        (
            vec![
                school("First", "38332"),
                school("Second", "38332"),
                school("Charles", "4912"),
            ],
            "ambiguous MilesplitSchool provider teamID",
        ),
        (
            vec![school("Leading zero", "038332"), school("Charles", "4912")],
            "missing exact positive MilesplitSchool provider teamID",
        ),
        (
            vec![wrong_namespace, school("Charles", "4912")],
            "missing exact positive MilesplitSchool provider teamID",
        ),
    ] {
        let (accumulated, _) = project(document()?, &schools, metadata()?)?;
        check!(eq;
            accumulated
                .performances
                .values()
                .map(|row| row.source_key.as_str())
                .collect::<std::collections::BTreeSet<_>>(),
            std::collections::BTreeSet::from(["milesplit_result:201782806"])
        );
        let retained = accumulated
            .retained
            .values()
            .find(|row| row["result_id"] == 201782263)
            .ok_or("unresolved original result")?;
        check!(eq; retained["disposition"], "retained_unresolved");
        check!(eq; retained["reasons"], json!([reason]));
        check!(eq; retained["source_athlete"]["id"], "14222592");
        let note: Value = serde_json::from_str(
            retained["evidence"]["note"]
                .as_str()
                .ok_or("source facts")?,
        )?;
        check!(eq; note["provider"]["mark"], "13-9");
    }
    Ok(())
}

#[test]
fn missing_or_invalid_published_cohort_retains_the_mark_without_reusing_other_row_cohort(
) -> TestResult {
    for token in [Value::Null, json!("9"), json!("0")] {
        let mut source = document()?;
        source["data"][0]["gradYear"] = token;
        let (accumulated, _) = project(
            source,
            &[school("Spann school", "38332"), school("Charles", "4912")],
            metadata()?,
        )?;
        check!(!accumulated
            .performances
            .values()
            .any(|row| row.source_key == "milesplit_result:201782263"));
        check!(accumulated
            .performances
            .values()
            .any(|row| row.source_key == "milesplit_result:201782277"));
        let SourceObservation::Athlete(observation) =
            &accumulated.observations["milesplit_result:201782263"]
        else {
            return Err("athlete".into());
        };
        check!(eq; observation.source_athlete_id, "14222592");
        check!(eq; observation.observed_grade, None);
        check!(eq;
            observation.profile_url.as_deref(),
            Some("https://www.milesplit.com/athletes/14222592-adelyn-spann")
        );
        let retained = accumulated
            .retained
            .values()
            .find(|row| row["result_id"] == 201782263)
            .ok_or("unsupported cohort observation")?;
        check!(eq; retained["disposition"], "retained_unresolved");
        check!(ne; retained["cohort"], "published");
    }
    Ok(())
}
