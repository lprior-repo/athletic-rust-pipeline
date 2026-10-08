use census_domain::model::{
    CanonicalEvent, CanonicalMeet, EventIdentity, EventKind, EventSpecification, Gender,
};
use census_domain::UsJurisdiction;

fn main() {
    for label in [
        "Boys Varsity Shot Put Finals",
        "Girls Results Triple Jump Finals",
        "Boys Varsity 4x400 Meter Relay Finals",
        "100 Meter Dash",
        "200 Meter Dash",
        "4x100 Meter Relay",
        "Shot Put",
        "5000 Meter Run",
        "100 Meters",
        "4x800 Relay",
    ] {
        let meet = CanonicalMeet::mint(
            Some(UsJurisdiction::Wisconsin),
            "2026-04-21",
            "Captured Meet",
            None,
        );
        let kind = EventKind::from_source_label(label);
        println!("{label}: kind={kind:?}");
        match EventSpecification::from_published_label(label, &kind) {
            Ok(specification) => {
                println!("  spec={specification:?}");
                let identity = EventIdentity {
                    meet: &meet,
                    kind: EventKind::Unmapped {
                        label: label.into(),
                    },
                    gender: Gender::Boys,
                    division: Some("2A"),
                    round: Some("Preliminaries"),
                };
                match CanonicalEvent::new(identity, specification) {
                    Ok(event) => println!("  mint OK id={:?}", event.id),
                    Err(error) => println!("  mint ERR: {error}"),
                }
            }
            Err(error) => println!("  parse ERR: {error}"),
        }
    }
}
