//! Mutation-killing tests for the canonical model in [`super`]: rendered ids, label mappings,
//! boundary months, folded names, the mailbox policy.

use super::*;

#[test]
fn display_impls_render_their_value() {
    let school = CanonicalSchool::mint("WI", "Abbotsford High School", "abbotsford");
    assert_eq!(school.to_string(), "sch_b5ea31ddfcd999ba");
    let kid = CanonicalAthlete::mint(&school, "Julian Aguilera", GradYear(2027), Gender::Boys);
    assert_eq!(kid.to_string(), "ath_77445d74c6dd8dbb");
    assert_eq!(kid.to_string(), kid.as_str(), "Display is the id itself");
    assert_eq!(Grade::new(9).unwrap().to_string(), "9");
    assert_eq!(Grade::new(12).unwrap().to_string(), "12");
    assert_eq!(GradYear::CO2027.to_string(), "2027");
    assert_eq!(GradYear(2030).to_string(), "2030");
}

#[test]
fn grad_year_get_reads_the_cohort_it_was_built_from() {
    let of = |grade, year| GradYear::of(Grade::new(grade).unwrap(), SchoolYear(year));
    for year in [2020, 2024, 2027, 2030, 2040] {
        assert_eq!(GradYear(year).get(), year);
        assert_eq!(GradYear::new(year).unwrap().get(), year);
    }
    assert_eq!(GradYear::CO2027.get(), 2027);
    assert_eq!(of(11, 2025), GradYear::CO2027);
    assert_eq!(of(12, 2025), GradYear(2026));
    assert_eq!(of(12, 2026), GradYear::CO2027);
    assert_eq!(of(11, 2025).get(), 2027);
    let observed = ObservedGrade {
        grade: Grade::new(9).unwrap(),
        school_year: SchoolYear(2026),
        source: SourceRef::id("milesplit_roster"),
    };
    assert_eq!(observed.grad_year(), GradYear(2030));
    assert_eq!(observed.school_year.short(), "2026-27");
}

#[test]
fn school_year_flips_at_august_first() {
    assert_eq!(SchoolYear::containing(2025, 1), SchoolYear(2024));
    assert_eq!(SchoolYear::containing(2025, 7), SchoolYear(2024));
    assert_eq!(SchoolYear::containing(2025, 8), SchoolYear(2025));
    assert_eq!(SchoolYear::containing(2025, 12), SchoolYear(2025));
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
    let school = CanonicalSchool::mint("WI", "Abbotsford", "abbotsford");
    let cohort = GradYear(2027);
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
    let a = CanonicalSchool::mint("WI", "Abbotsford High School", "abbotsford");
    let b = CanonicalSchool::mint("WI", "Abbotsford High School", "abbotsford");
    assert_eq!(a, b);
    assert!(a.as_str().starts_with("sch_"));
    let other = CanonicalSchool::mint("MN", "Abbotsford High School", "abbotsford");
    assert_ne!(a, other, "state participates in the natural key");
}

#[test]
fn meet_identity_is_date_and_name_scoped() {
    let a = CanonicalMeet::mint("WI", "2026-05-29", "D3 Sectional #3", None);
    let b = CanonicalMeet::mint("WI", "2026-05-29", "D3 sectional #3", None);
    let c = CanonicalMeet::mint("WI", "2026-05-30", "D3 Sectional #3", None);
    assert_eq!(a, b);
    assert_ne!(a, c);
    let venue = "La Crosse, WI";
    let d = CanonicalMeet::mint("WI", "2026-05-29", "D3 Sectional #3", Some(venue));
    assert_eq!(a, d, "venue spelling must not fork meet identity");
    let level = CompetitionLevel::Sectional;
    let built = CanonicalMeet::new("WI", "D3 Sectional #3", "2026-05-29", level);
    assert_eq!(built.id, a, "constructor and mint must agree");
}
