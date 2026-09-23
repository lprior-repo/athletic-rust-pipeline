//! Mutation-killing tests for the canonical model in [`super`]: rendered ids, label mappings,
//! boundary months, folded names, the mailbox policy.

use super::*;

#[test]
fn corpus_ids_keep_the_digest_they_were_minted_with() {
    // Pinned to the bytes the pre-`stable_key` mints produced for these fixtures (captured by running
    // the `Debug`-based mint over them), so every spelling below is a stored id that must not move.
    let school = CanonicalSchool::mint(
        UsJurisdiction::Wisconsin,
        "Abbotsford High School",
        "abbotsford",
    );
    assert_eq!(school.to_string(), "sch_b5ea31ddfcd999ba");
    let team = CanonicalTeam::mint(
        &school,
        Sport::OutdoorTrack,
        Gender::Boys,
        SchoolYear::new(2025).unwrap(),
    );
    assert_eq!(team.to_string(), "team_6a4d256c6bc97232");
    let xc = CanonicalTeam::mint(
        &school,
        Sport::CrossCountry,
        Gender::Girls,
        SchoolYear::new(2026).unwrap(),
    );
    assert_eq!(xc.to_string(), "team_86312fb5a8522a0f");
    let coach = CanonicalCoach::new(
        &school,
        "Dana Reed",
        Some(Sport::OutdoorTrack),
        Gender::Girls,
        CoachRole::HeadCoach,
    );
    assert_eq!(coach.id.to_string(), "coa_d53b5b0e0d2da942");
    let director = CanonicalCoach::new(
        &school,
        "Sam Okafor",
        None,
        Gender::Unknown,
        CoachRole::AthleticDirector,
    );
    assert_eq!(director.id.to_string(), "coa_67dcebf1343ccc3e");
    let meet = CanonicalMeet::mint(
        Some(UsJurisdiction::Wisconsin),
        "2026-05-30",
        "WIAA Division 2 State Championships",
        None,
    );
    assert_eq!(meet.to_string(), "meet_f661f872d6f53598");
    let hundred = CanonicalEvent::new(
        &meet,
        EventKind::Track100m,
        Gender::Boys,
        Some("D2"),
        Some("final"),
    );
    assert_eq!(hundred.id.to_string(), "evt_d1abc1f15ea80cc9");
    let unmapped = CanonicalEvent::new(
        &meet,
        EventKind::Unmapped {
            label: "Sprint Medley Relay".to_string(),
        },
        Gender::Girls,
        None,
        None,
    );
    assert_eq!(unmapped.id.to_string(), "evt_98381b783e2a0302");
    let athlete = CanonicalAthlete::mint(
        &school,
        "Julian Aguilera",
        GradYear::new(2027).unwrap(),
        Gender::Boys,
    );
    assert_eq!(athlete.to_string(), "ath_77445d74c6dd8dbb");
    let performance = CanonicalPerformance::mint(
        &athlete,
        &meet,
        &EventKind::Track100m,
        "2026-05-30",
        "wiaa_results:d2-100-final:julian-aguilera",
    );
    assert_eq!(performance.to_string(), "perf_bf5f67ddbd14bdf6");
}

/// Every [`EventKind`], so the spelling test below cannot silently skip one.
fn every_event_kind() -> Vec<EventKind> {
    let kinds = vec![
        EventKind::Track100m,
        EventKind::Track200m,
        EventKind::Track400m,
        EventKind::Track800m,
        EventKind::Track1600m,
        EventKind::Track3200m,
        EventKind::Track1Mile,
        EventKind::Track3000m,
        EventKind::Track5000m,
        EventKind::Track110mHurdles,
        EventKind::Track100mHurdles,
        EventKind::Track300mHurdles,
        EventKind::Track400mHurdles,
        EventKind::Track2000mSteeplechase,
        EventKind::Track3000mSteeplechase,
        EventKind::CrossCountry,
        EventKind::Relay4x100,
        EventKind::Relay4x200,
        EventKind::Relay4x400,
        EventKind::Relay4x800,
        EventKind::SprintMedley,
        EventKind::DistanceMedley,
        EventKind::HighJump,
        EventKind::LongJump,
        EventKind::TripleJump,
        EventKind::PoleVault,
        EventKind::ShotPut,
        EventKind::Discus,
        EventKind::Javelin,
        EventKind::Hammer,
        EventKind::WeightThrow,
        EventKind::Pentathlon,
        EventKind::Heptathlon,
        EventKind::Decathlon,
        EventKind::Unmapped {
            label: "Sprint Medley Relay".to_string(),
        },
    ];
    assert_eq!(
        kinds.len(),
        35,
        "extend this list when EventKind gains a variant"
    );
    kinds
}

#[test]
fn stable_key_spells_exactly_what_the_mint_hashed_before() {
    // The whole point of `stable_key` is that it is byte-identical to the `Debug` spelling the mints
    // hashed, for every value they can hash: exhaustive here, so a renamed variant or a re-derived
    // `Debug` cannot move a stored id unnoticed.
    let sports = [Sport::OutdoorTrack, Sport::IndoorTrack, Sport::CrossCountry];
    for sport in sports {
        assert_eq!(sport.stable_key(), format!("{sport:?}"), "sport spelling");
    }
    let genders = [Gender::Boys, Gender::Girls, Gender::Mixed, Gender::Unknown];
    for gender in genders {
        assert_eq!(
            gender.stable_key(),
            format!("{gender:?}"),
            "gender spelling"
        );
    }
    let timings = [TimingMethod::Fat, TimingMethod::Hand, TimingMethod::Unknown];
    for timing in timings {
        assert_eq!(
            timing.stable_key(),
            format!("{timing:?}"),
            "timing method spelling"
        );
    }
    // CanonicalEvent mint: the id is built from each input's stable key, so the spelling below is the
    // persistence contract, and `stable_key` above is what those inputs must keep spelling.
    let meet_id = CanonicalMeet::mint(None, "2026-06-01", "Invitational", None);
    let event = CanonicalEvent::new(
        &meet_id,
        EventKind::Track800m,
        Gender::Girls,
        Some("D1"),
        None,
    );
    let expected = Id::mint("evt", &[meet_id.as_str(), "Track800m", "f", "D1", ""]);
    assert_eq!(event.id, expected, "CanonicalEvent mint parity");
    let roles = [
        CoachRole::HeadCoach,
        CoachRole::AssistantCoach,
        CoachRole::AthleticDirector,
        CoachRole::Unknown,
    ];
    for role in roles {
        assert_eq!(
            role.stable_key(),
            format!("{role:?}"),
            "coach role spelling"
        );
    }
    for kind in every_event_kind() {
        assert_eq!(
            kind.stable_key(),
            format!("{kind:?}"),
            "event kind spelling"
        );
    }
    // A label that needs escaping: the mint hashed the escaped literal, and so must the key.
    let escaped = EventKind::Unmapped {
        label: "4x100 \"relay\"\\heat".to_string(),
    };
    assert_eq!(escaped.stable_key(), format!("{escaped:?}"));
}

#[test]
fn display_impls_render_their_value() {
    let school = CanonicalSchool::mint(
        UsJurisdiction::Wisconsin,
        "Abbotsford High School",
        "abbotsford",
    );
    assert_eq!(school.to_string(), "sch_b5ea31ddfcd999ba");
    let kid = CanonicalAthlete::mint(
        &school,
        "Julian Aguilera",
        GradYear::new(2027).unwrap(),
        Gender::Boys,
    );
    assert_eq!(kid.to_string(), "ath_77445d74c6dd8dbb");
    assert_eq!(kid.to_string(), kid.as_str(), "Display is the id itself");
    assert_eq!(Grade::new(9).unwrap().to_string(), "9");
    assert_eq!(Grade::new(12).unwrap().to_string(), "12");
    assert_eq!(GradYear::CO2027.to_string(), "2027");
    assert_eq!(GradYear::new(2030).unwrap().to_string(), "2030");
}

#[test]
fn grad_year_get_reads_the_cohort_it_was_built_from() {
    let of = |grade, year| GradYear::of(Grade::new(grade).unwrap(), SchoolYear::new(year).unwrap());
    for year in [2020, 2024, 2027, 2030, 2040] {
        assert_eq!(GradYear::new(year).unwrap().get(), year);
        assert_eq!(GradYear::new(year).unwrap().get(), year);
    }
    assert_eq!(GradYear::CO2027.get(), 2027);
    assert_eq!(of(11, 2025), GradYear::CO2027);
    assert_eq!(of(12, 2025), GradYear::new(2026).unwrap());
    assert_eq!(of(12, 2026), GradYear::CO2027);
    assert_eq!(of(11, 2025).get(), 2027);
    let observed = ObservedGrade {
        grade: Grade::new(9).unwrap(),
        school_year: SchoolYear::new(2026).unwrap(),
        source: SourceRef::id("milesplit_roster"),
    };
    assert_eq!(observed.grad_year(), GradYear::new(2030).unwrap());
    assert_eq!(observed.school_year.short(), "2026-27");
}

#[test]
fn school_year_flips_at_august_first() {
    assert_eq!(
        SchoolYear::containing(2025, 1),
        Some(SchoolYear::new(2024).unwrap())
    );
    assert_eq!(
        SchoolYear::containing(2025, 7),
        Some(SchoolYear::new(2024).unwrap())
    );
    assert_eq!(
        SchoolYear::containing(2025, 8),
        Some(SchoolYear::new(2025).unwrap())
    );
    assert_eq!(
        SchoolYear::containing(2025, 12),
        Some(SchoolYear::new(2025).unwrap())
    );
    // A date that steps back past the earliest season a source publishes is not a season.
    assert_eq!(SchoolYear::containing(SchoolYear::MIN_START_YEAR, 1), None);
}

#[test]
fn every_namespace_renders_and_classifies() {
    use SourceNamespace::*;
    let school = |a: String| AssociationSchool { association: a };
    let athlete = |a: String| AssociationAthlete { association: a };
    let team = |p: String| TimerTeam { provider: p };
    let timer = |p: String| TimerAthlete { provider: p };
    let meet = |p: String| TimerMeet { provider: p };
    // The two namespaces that are not supplied by our own adapters; `legacy_*` is retained
    // Athletic.net evidence, `net` is the live Athletic.net adapter.
    let old = |kind: String| LegacyAthleticNet { kind };
    let net = |kind: String| AthleticNet { kind };
    let cases: &[(SourceNamespace, &str, bool)] = &[
        (MilesplitSchool, "milesplit_school", true),
        (MilesplitTeam, "milesplit_team", true),
        (MilesplitAthlete, "milesplit_athlete", true),
        (MilesplitMeet, "milesplit_meet", true),
        (TfrrsTeam, "tfrrs_team", true),
        (TfrrsAthlete, "tfrrs_athlete", true),
        (DirectAthleticsTeam, "direct_athletics_team", true),
        (DirectAthleticsAthlete, "direct_athletics_athlete", true),
        (school("wiaa".into()), "association_school:wiaa", true),
        (athlete("ihsa".into()), "association_athlete:ihsa", true),
        (team("pttiming".into()), "timer_team:pttiming", true),
        (timer("wayzata".into()), "timer_athlete:wayzata", true),
        (meet("raceday".into()), "timer_meet:raceday", true),
        (old("athlete".into()), "legacy_athletic_net:athlete", false),
        (net("school".into()), "athleticnet:school", false),
        (Other("custom_provider".into()), "custom_provider", true),
    ];
    for (namespace, rendered, is_core) in cases {
        assert_eq!(namespace.to_string(), *rendered);
        assert_eq!(namespace.is_core(), *is_core, "{namespace}");
    }
}

/// Every arm of `EventKind::from_source_label` as `label=Variant`, plus a second spelling for arms
/// whose normalization is worth pinning; a label repeated with a different variant exercises
/// [`EventKind::normalized_label`].
const EVENT_LABELS: &str = "\
100m=Track100m;100 Meters=Track100m;200m=Track200m;400m=Track400m;800m=Track800m;\
800-metre=Track800m;1600m=Track1600m;3200m=Track3200m;1mile=Track1Mile;1 Mile=Track1Mile;\
3000m=Track3000m;3k=Track3000m;5000m=Track5000m;110mh=Track110mHurdles;\
110m Hurdles=Track110mHurdles;110mH=Track110mHurdles;100mh=Track100mHurdles;\
300mh=Track300mHurdles;400mh=Track400mHurdles;2000msteeplechase=Track2000mSteeplechase;\
2K Steeplechase=Track2000mSteeplechase;3000msteeplechase=Track3000mSteeplechase;\
3000msteeple=Track3000mSteeplechase;crosscountry=CrossCountry;xc=CrossCountry;\
Cross-Country=CrossCountry;4x100m=Relay4x100;4x100 Relay=Relay4x100;4x200m=Relay4x200;\
4x400m=Relay4x400;4x400 Meter Relay=Relay4x400;4x800m=Relay4x800;sprintmedley=SprintMedley;\
smr=SprintMedley;distancemedley=DistanceMedley;Distance Medley=DistanceMedley;\
highjump=HighJump;High Jump=HighJump;longjump=LongJump;triplejump=TripleJump;\
polevault=PoleVault;shotput=ShotPut;shot_put=ShotPut;discus=Discus;javelin=Javelin;\
hammer=Hammer;weightthrow=WeightThrow;pentathlon=Pentathlon;heptathlon=Heptathlon;\
decathlon=Decathlon";

/// Labels that must be field events, relays, or neither; `3,200m` is the unmapped fallback.
const FIELD_LABELS: &str = "highjump longjump triplejump polevault shotput discus javelin \
                            hammer weightthrow pentathlon heptathlon decathlon";
const RELAY_LABELS: &str = "4x100m 4x200m 4x400m 4x800m sprintmedley distancemedley";
const PLAIN_LABELS: &str = "100m 200m 400m 800m 1600m 3200m 1mile 3000m 5000m 110mh 100mh \
                            300mh 400mh 2000msteeplechase 3000msteeplechase crosscountry 3,200m";

#[test]
fn every_event_label_maps_to_its_kind() {
    for pair in EVENT_LABELS.split(';') {
        let (label, want) = pair.split_once('=').unwrap();
        let got = format!("{:?}", EventKind::from_source_label(label));
        assert_eq!(got, want, "label {label:?}");
    }
    // An unrecognized label is preserved as evidence, whitespace trimmed.
    let unmapped = |l: String| EventKind::Unmapped { label: l };
    let got = EventKind::from_source_label(" 3,200m ");
    assert_eq!(got, unmapped("3,200m".into()));
}

#[test]
fn field_and_relay_flags_match_the_kind() {
    for label in FIELD_LABELS.split_whitespace() {
        let kind = EventKind::from_source_label(label);
        assert!(kind.is_field() && !kind.is_relay(), "{label}");
    }
    for label in RELAY_LABELS.split_whitespace() {
        let kind = EventKind::from_source_label(label);
        assert!(kind.is_relay() && !kind.is_field(), "{label}");
    }
    for label in PLAIN_LABELS.split_whitespace() {
        let kind = EventKind::from_source_label(label);
        assert!(!kind.is_field() && !kind.is_relay(), "{label}");
    }
}

#[test]
fn milesplit_gender_aliases_parse() {
    // The trailing separator on the last row is the empty string: no side at all.
    for (aliases, gender) in [
        ("m|male|boys|boy| BOYS |Male", Gender::Boys),
        ("f|female|girls|girl|Female", Gender::Girls),
        ("mixed|men|co-ed|", Gender::Unknown),
    ] {
        for alias in aliases.split('|') {
            assert_eq!(Gender::parse_milesplit(alias), gender, "side {alias:?}");
        }
    }
}

#[test]
fn athlete_ids_separate_gender_sides_and_ignore_spacing() {
    let school = CanonicalSchool::mint(UsJurisdiction::Wisconsin, "Abbotsford", "abbotsford");
    let cohort = GradYear::new(2027).unwrap();
    let mint = |name: &str, gender| CanonicalAthlete::mint(&school, name, cohort, gender);
    let boys = mint("Julian Aguilera", Gender::Boys);
    // Name spelling is normalized away; the gender side and cohort are part of the key.
    assert_eq!(boys, mint("  Julian   Aguilera  ", Gender::Boys));
    assert_eq!(boys.to_string(), "ath_77445d74c6dd8dbb");
    let girls = mint("Julian Aguilera", Gender::Girls);
    assert_eq!(girls.to_string(), "ath_14ddac749d00a84e");
    let other = mint("Julian Aguilera", Gender::Mixed);
    assert_eq!(other, mint("Julian Aguilera", Gender::Unknown));
    assert_ne!(boys, girls);
    assert_ne!(boys, other);
    assert_ne!(girls, other);
}

#[test]
fn mark_raw_reports_the_published_value_or_its_unit() {
    let imperial = |feet: &str, metres| Mark::FieldImperial {
        feet_mark: feet.into(),
        metres,
    };
    let cases: &[(Mark, &str)] = &[
        (Mark::Raw("41-06.5".into()), "41-06.5"),
        (Mark::Raw(String::new()), ""),
        (Mark::Raw(" ".into()), " "),
        (Mark::TimeSeconds(10.94), "time"),
        (Mark::DistanceMetres(1.73), "distance"),
        (imperial("5' 4\"", 1.63), "field"),
        (Mark::Points(8421.0), "points"),
    ];
    for (mark, raw) in cases {
        assert_eq!(mark.raw(), *raw);
    }
}

#[test]
fn professional_email_boundaries() {
    // One empty half is enough to drop a malformed address.
    assert_eq!(professional_email("@ofsd.k12.wi.us"), None);
    assert_eq!(professional_email("coach@"), None);
    assert_eq!(professional_email(""), None);
    assert_eq!(professional_email("no-at-sign"), None);
    // Personal mailboxes, exact and in a subdomain, in any case.
    for domain in ["gmail.com", "GMAIL.com", "sub.gmail.com", "proton.me"] {
        let mailbox = format!("coach@{domain}");
        assert_eq!(professional_email(&mailbox), None, "{domain}");
    }
    // School mailboxes survive, including domains that merely contain a consumer name.
    for domain in ["notgmail.com", "llhs.org", "gmail.com.evil.org"] {
        let address = format!("ad@{domain}");
        let kept = professional_email(&address);
        assert_eq!(kept.as_deref(), Some(address.as_str()), "{domain}");
    }
    let trimmed = professional_email(" jstoik@ofsd.k12.wi.us ");
    assert_eq!(trimmed.as_deref(), Some("jstoik@ofsd.k12.wi.us"));
    let caps = professional_email("AD@LLHS.ORG");
    assert_eq!(caps.as_deref(), Some("AD@LLHS.ORG"));
}

#[test]
fn normalize_name_turns_separators_into_one_space() {
    for (raw, expected) in [
        ("A B", "a b"),
        ("a-b", "a b"),
        ("a'b", "a b"),
        ("a.b", "a b"),
        ("a,b", "a b"),
        ("a/b", "a b"),
        ("a_b", "ab"),
        ("Plain ASCII 123", "plain ascii 123"),
        ("  Glencoe-Silver   Lake HS ", "glencoe silver lake"),
    ] {
        assert_eq!(normalize_name(raw), expected, "raw {raw:?}");
    }
}

#[test]
fn normalize_name_folds_every_accented_arm() {
    let accented = "áàâäãåāéèêëēęíìîïīóòôöõōøúùûüūñńçćšśžźżýÿłæœß";
    // One ASCII letter per arm, in the order the arms are written: a e i o u n c s z y l a o s.
    let folded = "aaaaaaaeeeeeeiiiiiooooooouuuuunnccsszzzyylaos";
    assert_eq!(accented.chars().count(), folded.chars().count());
    for (raw, expected) in accented.chars().zip(folded.chars()) {
        let name = raw.to_string();
        assert_eq!(normalize_name(&name), expected.to_string(), "{raw}");
    }
    // Characters outside the table are dropped; their ASCII neighbors survive.
    assert_eq!(normalize_name("Řeřicha School"), "eicha");
    assert_eq!(normalize_name("César Chávez School"), "cesar chavez");
}

#[test]
fn normalize_name_strips_school_suffixes_only_from_longer_names() {
    for (raw, expected) in [
        ("Abbotsford High School", "abbotsford"),
        ("Abbotsford Highschool", "abbotsford"),
        ("Abbotsford HS", "abbotsford"),
        ("Abbotsford School", "abbotsford"),
        ("Glencoe Sr High", "glencoe"),
        ("Glencoe Senior High", "glencoe"),
        ("Abbotsford Academy", "abbotsford academy"),
        // A name that is nothing but a suffix keeps its tokens; stripping it would leave "".
        ("HS", "hs"),
        ("Sr High", "sr high"),
    ] {
        assert_eq!(normalize_name(raw), expected, "raw {raw:?}");
    }
}

#[test]
fn normalize_name_reaches_a_fixpoint_on_repeated_suffixes() {
    // A trailing suffix that survives one pass would mint a different `SchoolId` for a source that
    // re-normalizes a name another source already normalized.
    for (raw, expected) in [
        ("X School School", "x"),
        ("Center Grove High School High School", "center grove"),
        ("A B HS School", "a b"),
        ("A B School Sr High", "a b"),
    ] {
        assert_eq!(normalize_name(raw), expected, "raw {raw:?}");
    }
    for raw in [
        "Abbotsford High School",
        "X School School",
        "A B School Sr High",
        "HS",
        "",
    ] {
        let once = normalize_name(raw);
        assert_eq!(normalize_name(&once), once, "not idempotent on {raw:?}");
    }
}

#[test]
fn flip_last_first_handles_both_shapes() {
    assert_eq!(flip_last_first("Aguilera, Julian"), "Julian Aguilera");
    assert_eq!(flip_last_first(" Aguilera , Julian "), "Julian Aguilera");
    assert_eq!(flip_last_first("Julian Aguilera"), "Julian Aguilera");
    // A comma with an empty half is not a "Last, First" roster pair.
    assert_eq!(flip_last_first("Aguilera,"), "Aguilera,");
    assert_eq!(flip_last_first(", Julian"), ", Julian");
    assert_eq!(flip_last_first(","), ",");
}

#[test]
fn ids_are_deterministic_and_prefixed() {
    let a = CanonicalSchool::mint(
        UsJurisdiction::Wisconsin,
        "Abbotsford High School",
        "abbotsford",
    );
    let b = CanonicalSchool::mint(
        UsJurisdiction::Wisconsin,
        "Abbotsford High School",
        "abbotsford",
    );
    assert_eq!(a, b);
    assert!(a.as_str().starts_with("sch_"));
    let other = CanonicalSchool::mint(
        UsJurisdiction::Minnesota,
        "Abbotsford High School",
        "abbotsford",
    );
    assert_ne!(a, other, "state participates in the natural key");
}

#[test]
fn meet_identity_is_date_and_name_scoped() {
    let a = CanonicalMeet::mint(
        Some(UsJurisdiction::Wisconsin),
        "2026-05-29",
        "D3 Sectional #3",
        None,
    );
    let b = CanonicalMeet::mint(
        Some(UsJurisdiction::Wisconsin),
        "2026-05-29",
        "D3 sectional #3",
        None,
    );
    let c = CanonicalMeet::mint(
        Some(UsJurisdiction::Wisconsin),
        "2026-05-30",
        "D3 Sectional #3",
        None,
    );
    assert_eq!(a, b);
    assert_ne!(a, c);
    let venue = "La Crosse, WI";
    let d = CanonicalMeet::mint(
        Some(UsJurisdiction::Wisconsin),
        "2026-05-29",
        "D3 Sectional #3",
        Some(venue),
    );
    assert_eq!(a, d, "venue spelling must not fork meet identity");
    let level = CompetitionLevel::Sectional;
    let built = CanonicalMeet::new(
        Some(UsJurisdiction::Wisconsin),
        "D3 Sectional #3",
        "2026-05-29",
        level,
    );
    assert_eq!(built.id, a, "constructor and mint must agree");
    // The literal is the pre-cutover id, computed independently from
    // `sha256("meet" 0x1f "WI" 0x1f date 0x1f normalized)`: the jurisdiction participates as its
    // USPS code, so typing the parameter re-mints nothing already in the store.
    assert_eq!(a.to_string(), "meet_019891d607bdeb6c");
    assert_eq!(built.state, Some(UsJurisdiction::Wisconsin));
}

/// The unresolved bucket is not a jurisdiction: `None` and the legacy `??` row are one meet.
#[test]
fn an_unplaced_meet_keeps_the_legacy_unknown_state_id() {
    let unplaced = CanonicalMeet::mint(None, "2026-05-29", "D3 Sectional #3", None);
    // Literal from `sha256("meet" 0x1f "??" 0x1f date 0x1f normalized)`: the same bytes the
    // free-string era hashed for this row, so the 442 stored `"state":"??"` meets are not orphaned.
    assert_eq!(unplaced.to_string(), "meet_ddb3074882d2561f");
    let placed = CanonicalMeet::mint(
        Some(UsJurisdiction::Wisconsin),
        "2026-05-29",
        "D3 Sectional #3",
        None,
    );
    assert_ne!(
        unplaced, placed,
        "placing the venue is what separates a coverage gap from a known jurisdiction"
    );
    assert_eq!(
        CanonicalMeet::new(
            None,
            "D3 Sectional #3",
            "2026-05-29",
            CompetitionLevel::Sectional
        )
        .state,
        None
    );
}

/// The legacy sentinel is the only string that may decode to an absent jurisdiction.
#[test]
fn a_legacy_unresolved_meet_state_decodes_to_none() {
    use serde::de::value::{Error, StrDeserializer};
    let decode =
        |raw: &str| super::meet::deserialize_meet_state(StrDeserializer::<Error>::new(raw));
    assert_eq!(decode(MEET_STATE_UNRESOLVED), Ok(None));
    assert_eq!(decode("WI"), Ok(Some(UsJurisdiction::Wisconsin)));
    assert_eq!(decode("wi"), Ok(Some(UsJurisdiction::Wisconsin)));
    assert!(decode("PR").is_err(), "a territory is not a jurisdiction");
}

#[test]
fn school_state_is_stored_as_the_validated_jurisdiction() {
    let (school, id) = CanonicalSchool::new(
        UsJurisdiction::Wisconsin,
        "Abbotsford High School",
        "abbotsford",
    );
    assert_eq!(school.state, Some(UsJurisdiction::Wisconsin));
    // Same id as the free-string era: the natural key hashes the jurisdiction code.
    assert_eq!(id.to_string(), "sch_b5ea31ddfcd999ba");
    // The jurisdiction's code is what a report or key renderer asks for, and it is the wire form.
    assert_eq!(school.state.map(UsJurisdiction::code), Some("WI"));
}
