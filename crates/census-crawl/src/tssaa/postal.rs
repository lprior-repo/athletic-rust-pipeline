use census_domain::school_directory::PostalAddress;
use census_domain::UsJurisdiction;

use crate::directory::{self, compile_pattern, group, line_of, AddressParts, ReadOutcome};
use crate::CrawlResult;

use super::fields;

pub(super) fn parse_addresses(
    text: &str,
    outcome: &mut ReadOutcome,
) -> CrawlResult<Vec<(String, PostalAddress)>> {
    let blocks = compile_pattern(
        r"(?is)<strong>\s*(Mailing|Physical|Shipping) Address:\s*</strong>(.*?)</div>",
        "tssaa postal blocks",
    )?;
    let breaks = compile_pattern(r"(?i)<br\s*/?>", "tssaa address breaks")?;
    let tag = compile_pattern(fields::TAG, "tssaa address tags")?;
    let locality = compile_pattern(r"^(.+),\s*([A-Z]{2})(?:\s+(\S+))?$", "tssaa locality")?;
    let addresses = blocks
        .captures_iter(text)
        .filter_map(|captures| {
            let line = captures.get(0).map_or(1, |row| line_of(text, row.start()));
            let lines: Vec<_> = breaks
                .split(group(&captures, 2))
                .map(|raw| fields::strip_tags(&tag, raw))
                .filter(|line| !line.is_empty())
                .collect();
            match address_parts(&lines, &locality) {
                Ok(address) => Some((group(&captures, 1).to_string(), address)),
                Err(detail) => {
                    outcome.note(line, "address", detail);
                    None
                }
            }
        })
        .collect();
    Ok(addresses)
}

fn address_parts(lines: &[String], locality: &regex::Regex) -> Result<PostalAddress, String> {
    let (Some(street), Some(last)) = (lines.first(), lines.last()) else {
        return Err("empty published postal block".to_string());
    };
    if !(2..=3).contains(&lines.len()) {
        return Err("postal block requires street, optional line 2, and locality".to_string());
    }
    let captures = locality
        .captures(last)
        .ok_or_else(|| "published postal locality is malformed".to_string())?;
    let state = group(&captures, 2);
    if UsJurisdiction::parse(state) != Some(UsJurisdiction::Tennessee) {
        return Err(format!("published postal jurisdiction {state} is not TN"));
    }
    let line2 = if lines.len() == 3 {
        lines.get(1).map_or("", String::as_str)
    } else {
        ""
    };
    directory::postal_address(AddressParts {
        street,
        line2,
        city: group(&captures, 1),
        state,
        zip: group(&captures, 3),
        plus4: "",
    })
    .map_err(|failure| failure.detail)?
    .ok_or_else(|| "empty published postal address".to_string())
}
