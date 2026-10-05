use super::*;

fn candidate(name: &str) -> AthleteCandidateId {
    AthleteCandidateId::mint("can", &[name])
}

#[test]
fn joining_in_either_order_roots_at_the_smaller_candidate() -> Result<(), Box<dyn std::error::Error>>
{
    let left = candidate("alpha");
    let right = candidate("beta");
    let smaller = left.clone().min(right.clone());
    let mut forward = IdentityAliases::default();
    forward.join(&left, &right)?;
    let mut backward = IdentityAliases::default();
    backward.join(&right, &left)?;
    check!(eq; forward.root(&left)?.as_str(), smaller.as_str());
    check!(eq; backward.root(&left)?.as_str(), smaller.as_str());
    check!(eq; forward.flattened()?, backward.flattened()?);
    Ok(())
}

#[test]
fn a_smaller_candidate_joined_last_becomes_the_single_root(
) -> Result<(), Box<dyn std::error::Error>> {
    let first = candidate("aa");
    let second = candidate("bb");
    let third = candidate("cc");
    let mut smaller = first.clone();
    for other in [&second, &third] {
        smaller = smaller.min(other.clone());
    }
    let mut aliases = IdentityAliases::default();
    aliases.join(&second, &third)?;
    aliases.join(&first, &third)?;
    for member in [&first, &second, &third] {
        check!(eq; aliases.root(member)?.as_str(), smaller.as_str());
    }
    Ok(())
}

#[test]
fn every_join_preserves_the_strictly_decreasing_parent_invariant(
) -> Result<(), Box<dyn std::error::Error>> {
    let members = [
        candidate("p"),
        candidate("q"),
        candidate("r"),
        candidate("s"),
    ];
    let mut aliases = IdentityAliases::default();
    for (index, left) in members.iter().enumerate() {
        for right in members.iter().skip(index.saturating_add(1)) {
            aliases.join(left, right)?;
        }
    }
    let expected = members
        .iter()
        .min()
        .cloned()
        .ok_or(IdentityError::UnknownSubject("empty fixture".to_owned()))?;
    for member in &members {
        check!(eq; aliases.root(member)?.as_str(), expected.as_str());
    }
    for (child, parent) in &aliases.flattened()? {
        check!(parent < child);
    }
    Ok(())
}

#[test]
fn a_corrupt_parent_chain_is_refused_instead_of_merging() {
    let smaller = candidate("x");
    let larger = candidate("y");
    let (smaller, larger) = if smaller < larger {
        (smaller, larger)
    } else {
        (larger, smaller)
    };
    let mut aliases = IdentityAliases::default();
    aliases.parents.insert(smaller.clone(), larger.clone());
    assert!(matches!(
        aliases.root(&smaller),
        Err(IdentityError::AliasCycle)
    ));
    assert!(matches!(
        aliases.flattened(),
        Err(IdentityError::AliasCycle)
    ));
}
