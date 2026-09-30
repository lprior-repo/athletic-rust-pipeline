use super::*;

#[test]
fn every_namespace_renders_and_classifies() {
    use SourceNamespace::*;
    let school = |a: String| AssociationSchool { association: a };
    let athlete = |a: String| AssociationAthlete { association: a };
    let team = |p: String| TimerTeam { provider: p };
    let timer = |p: String| TimerAthlete { provider: p };
    let meet = |p: String| TimerMeet { provider: p };
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
    let unmapped_label = |l: String| EventKind::Unmapped { label: l };
    let got = EventKind::from_source_label(" 3,200m ");
    assert_eq!(got, EventKind::Track3200m);
    let got = EventKind::from_source_label("Flight 1 of 1");
    assert_eq!(got, unmapped_label("Flight 1 of 1".into()));
}

const WRAPPED_LABELS: &str = "\
Girls Results Triple Jump Finals=TripleJump;Boys Varsity Shot Put Finals=ShotPut;\
Girls Varsity Discus Finals=Discus;Boys Results Long Jump Finals=LongJump;\
Boys Junior Varsity Shot Put Finals=ShotPut;Boys Varsity Shot Put Preliminaries=ShotPut;\
Boys High School Shot Put Finals=ShotPut;Girls' 300 Hurdles=Track300mHurdles;\
Girls' Long Jump 3A=LongJump;ASICS Boys 1 Mile Finals=Track1Mile;\
HS Boys 1600m En Route Finals=Track1600m;Mens HS Discus Finals=Discus;\
Boys 100 Meter Dash=Track100m;5,000 Meters=Track5000m;Womens HS Shot Put Finals=ShotPut;\
Boys Varsity 4x400 Meter Relay Finals=Relay4x400;Girls' Javelin 6A=Javelin;\
Girls Javelin=Javelin;Boys 6A Javelin=Javelin";

#[test]
fn wrapped_event_labels_map_to_their_kind() {
    for pair in WRAPPED_LABELS.split(';') {
        let (label, want) = pair.split_once('=').unwrap();
        let got = format!("{:?}", EventKind::from_source_label(label));
        assert_eq!(got, want, "label {label:?}");
    }
}

#[test]
fn structural_labels_stay_unmapped() {
    for label in ["Flight 1 of 1", "Section 2 of 4", "Compiled", "Overall"] {
        let got = EventKind::from_source_label(label);
        assert!(
            matches!(got, EventKind::Unmapped { .. }),
            "{label} unexpectedly mapped to {got:?}"
        );
    }
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
