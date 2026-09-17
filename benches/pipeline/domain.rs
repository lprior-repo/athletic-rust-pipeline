use super::support::{ready, Fixture, RESULT_COUNT};
use athletic_rust_pipeline::{
    domain::{
        candidate::CandidateEvidence,
        decision::{assess, SearchCompleteness},
        evidence::Sport,
        identity::{AthleteId, ProfileUrl},
        marks::{best_performance, EventName, Performance},
        performance_evidence::summarize_performances,
    },
    profile::{merge_profiles, parse_bio, parse_profile_html},
    search::{parse_page, query_plan},
};
use criterion::{BatchSize, Criterion, Throughput};
use std::hint::black_box;

pub fn benchmark(c: &mut Criterion) {
    let fixture = ready(Fixture::new());
    benchmark_search(c, &fixture);
    benchmark_profiles(c, &fixture);
    benchmark_identity_and_decision(c, &fixture);
    benchmark_marks(c, &fixture);
}

fn benchmark_search(c: &mut Criterion, fixture: &Fixture) {
    let mut group = c.benchmark_group("search");
    group.throughput(Throughput::Bytes(ready(u64::try_from(
        fixture.search_body.len(),
    ))));
    group.bench_function("envelope_parse", |b| {
        b.iter_batched(
            || fixture.evidence_digest.clone(),
            |digest| {
                black_box(ready(parse_page(
                    &fixture.search_query,
                    0,
                    digest,
                    &fixture.search_body,
                )))
            },
            BatchSize::SmallInput,
        )
    });
    group.throughput(Throughput::Elements(1));
    group.bench_function("query_plan", |b| {
        b.iter(|| black_box(ready(query_plan(&fixture.records[0]))))
    });
    group.finish();
}

fn benchmark_profiles(c: &mut Criterion, fixture: &Fixture) {
    let mut group = c.benchmark_group("profile");
    group.throughput(Throughput::Bytes(ready(u64::try_from(
        fixture.profile_bio.len(),
    ))));
    group.bench_function("bio_parse", |b| {
        b.iter_batched(
            || (ready(AthleteId::new(123)), fixture.evidence_digest.clone()),
            |(id, digest)| {
                black_box(ready(parse_bio(
                    id,
                    Sport::TrackField,
                    digest,
                    &fixture.profile_bio,
                )))
            },
            BatchSize::SmallInput,
        )
    });
    group.throughput(Throughput::Bytes(ready(u64::try_from(
        fixture.profile_html.len(),
    ))));
    group.bench_function("html_parse", |b| {
        b.iter_batched(
            || (ready(AthleteId::new(123)), fixture.evidence_digest.clone()),
            |(id, digest)| black_box(ready(parse_profile_html(id, digest, &fixture.profile_html))),
            BatchSize::SmallInput,
        )
    });
    group.throughput(Throughput::Elements(ready(u64::try_from(
        fixture.profile.results.len(),
    ))));
    let duplicate = fixture.profile.clone();
    group.bench_function("merge_profiles", |b| {
        b.iter_batched(
            || (duplicate.clone(), duplicate.clone()),
            |(left, right)| black_box(ready(merge_profiles(left, right))),
            BatchSize::SmallInput,
        )
    });
    group.finish();
}

fn benchmark_identity_and_decision(c: &mut Criterion, fixture: &Fixture) {
    let mut group = c.benchmark_group("identity");
    group.bench_function("profile_url_parse", |b| {
        b.iter(|| {
            black_box(ready(ProfileUrl::parse(
                "https://www.athletic.net/athlete/123/track-and-field/all",
            )))
        })
    });
    let profiles = std::slice::from_ref(&fixture.profile);
    let complete = SearchCompleteness::Complete {
        evidence: fixture.evidence_digest.clone(),
    };
    group.bench_function("decision_assess", |b| {
        b.iter_batched(
            || complete.clone(),
            |search| {
                black_box(ready(assess(
                    &fixture.records[0],
                    profiles.iter().map(CandidateEvidence::Complete),
                    search,
                )))
            },
            BatchSize::SmallInput,
        )
    });
    group.finish();
}

fn benchmark_marks(c: &mut Criterion, fixture: &Fixture) {
    let event = ready(EventName::parse("100 Meters"));
    let raw_marks = (0..RESULT_COUNT)
        .map(|index| format!("10.{:02}", 50 - (index % 40)))
        .collect::<Vec<_>>();
    let performances = raw_marks
        .iter()
        .map(|raw| ready(Performance::parse(&event, raw)))
        .collect::<Vec<_>>();
    let mut group = c.benchmark_group("marks");
    group.throughput(Throughput::Elements(ready(u64::try_from(raw_marks.len()))));
    group.bench_function("parse_and_retain", |b| {
        b.iter(|| {
            black_box(ready(
                raw_marks
                    .iter()
                    .map(|raw| Performance::parse(&event, raw))
                    .collect::<Result<Vec<_>, _>>(),
            ))
        })
    });
    group.bench_function("best_performance", |b| {
        b.iter(|| black_box(best_performance(&event, &performances)))
    });
    group.bench_function("observed_best_summary", |b| {
        b.iter(|| black_box(summarize_performances(&fixture.profile.results)))
    });
    group.finish();
}
