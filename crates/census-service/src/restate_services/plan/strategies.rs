#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SourceStrategy {
    Teams,
    Meets,
    Results,
    Gap(&'static str),
}

pub(super) fn strategy(slug: &str) -> SourceStrategy {
    if super::super::teams_arms::arm_for(slug).is_some() {
        return SourceStrategy::Teams;
    }
    if super::super::meets_arms::arm_for(slug).is_some() {
        return SourceStrategy::Meets;
    }
    if super::super::results_arms::arm_for(slug).is_some() {
        return SourceStrategy::Results;
    }
    SourceStrategy::Gap(match slug {
        "athleticlive" => {
            "qualified artifact collector lacks durable jurisdiction artifact discovery"
        }
        "athleticlive_athletes" => {
            "qualified athlete-index collector lacks bounded jurisdiction seed discovery"
        }
        "tfrrs" => "qualified collector lacks durable jurisdiction list frontier discovery",
        "coach_contacts" => {
            "qualified contact import lacks a durable jurisdiction research frontier"
        }
        "nces" => "qualified school corpus collector lacks durable artifact selection",
        "state_ed" => "qualified state directory lacks a durable jurisdiction acquisition stage",
        _ => "registry family has no implemented durable execution strategy",
    })
}
