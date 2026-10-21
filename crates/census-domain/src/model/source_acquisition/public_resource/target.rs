use super::super::scalar;
use super::{CaptureDomainError, PublicSourceResourceV1, Wire};
use crate::UsJurisdiction;

impl PublicSourceResourceV1 {
    pub fn validate_target(
        &self,
        source_site: UsJurisdiction,
        target_url: &str,
    ) -> Result<(), CaptureDomainError> {
        let parsed = parse_target(target_url)?;
        let matches = match &self.0 {
            Wire::SchoolCalendar { origin, .. } => {
                parsed.origin().ascii_serialization() == *origin && calendar_path(parsed.path())
            }
            Wire::Robots { origin } => {
                parsed.origin().ascii_serialization() == *origin && parsed.path() == "/robots.txt"
            }
            resource => {
                let host = format!("{}.milesplit.com", source_site.code().to_ascii_lowercase());
                parsed.host_str() == Some(host.as_str()) && native_path(resource, parsed.path())
            }
        };
        if !matches {
            return Err(CaptureDomainError::RequestTargetMismatch);
        }
        Ok(())
    }
}

fn parse_target(value: &str) -> Result<url::Url, CaptureDomainError> {
    let parsed = scalar::url(value)?;
    if parsed.scheme() != "https"
        || parsed.port().is_some()
        || parsed.query().is_some()
        || parsed.as_str() != value
        || value.bytes().any(|byte| matches!(byte, b'%' | b'\\'))
    {
        return Err(CaptureDomainError::RequestTargetMismatch);
    }
    Ok(parsed)
}

fn native_path(resource: &Wire, path: &str) -> bool {
    match resource {
        Wire::MileSplitTeamDirectory {} => path == "/teams",
        Wire::MileSplitAthleteProfile { athlete_id } => path
            .strip_prefix("/athletes/")
            .is_some_and(|segment| native_segment(segment, athlete_id)),
        Wire::MileSplitTeamRoster { team_id } => path
            .strip_prefix("/teams/")
            .and_then(|tail| tail.strip_suffix("/roster"))
            .is_some_and(|segment| native_segment(segment, team_id)),
        Wire::MileSplitTeamMeetHistory { team_id } => path
            .strip_prefix("/teams/")
            .and_then(|tail| tail.strip_suffix("/meets"))
            .is_some_and(|segment| native_segment(segment, team_id)),
        Wire::MileSplitMeetTeam { meet_id, team_id } => path
            .strip_prefix("/meets/")
            .and_then(|tail| tail.split_once("/teams/"))
            .is_some_and(|(meet, team)| native_segment(meet, meet_id) && team == team_id),
        Wire::MileSplitRawResults { meet_id, result_id } => path
            .strip_prefix("/meets/")
            .and_then(|tail| tail.strip_suffix("/raw"))
            .and_then(|tail| tail.split_once("/results/"))
            .is_some_and(|(meet, result)| native_segment(meet, meet_id) && result == result_id),
        Wire::SchoolCalendar { .. } | Wire::Robots { .. } => false,
    }
}

fn native_segment(segment: &str, id: &str) -> bool {
    if segment == id {
        return true;
    }
    segment.split_once('-').is_some_and(|(observed_id, slug)| {
        observed_id == id
            && slug.bytes().any(|byte| byte.is_ascii_alphanumeric())
            && slug
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    })
}

fn calendar_path(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    ![
        "login", "log-in", "log_in", "logon", "log-on", "log_on", "signin", "sign-in", "sign_in",
        "signon", "sign-on", "sign_on", "logout", "log-out", "log_out", "signout", "sign-out",
        "sign_out", "auth", "account", "session", "sso", "saml", "password", "register", "admin",
    ]
    .iter()
    .any(|part| lower.contains(*part))
}
