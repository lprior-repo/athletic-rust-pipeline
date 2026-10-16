use super::*;

pub(super) fn head(
    writer: &mut impl Write,
    view: &StoreSnapshot<'_>,
    accepted: &AcceptedPublicIdentitySnapshot,
    inventory: &AttachmentInventoryV1,
) -> StoreResult<()> {
    sink::write(writer, b"{\"schema\":")?;
    sink::json(writer, SCHEMA)?;
    sink::write(writer, b",\"store\":")?;
    sink::json(writer, accepted.store_identity().as_str())?;
    sink::write(writer, b",\"snapshot_sequence\":")?;
    sink::json(writer, &view.sequence())?;
    sink::write(writer, b",\"public_head\":")?;
    sink::json(writer, accepted.head())?;
    sink::write(writer, b",\"inventory\":")?;
    sink::json(writer, inventory)?;
    sink::write(writer, b",\"prefixes\":[")
}

pub(super) fn prefix(
    writer: &mut impl Write,
    index: usize,
    summary: &Summary<'_>,
) -> StoreResult<()> {
    if index != 0 {
        sink::write(writer, b",")?;
    }
    sink::write(writer, b"{\"kind\":")?;
    let kind = match summary.group {
        Group::Attachment => "attachment",
        Group::Receipts => "receipts",
        Group::External(_) => "external",
    };
    sink::json(writer, kind)?;
    sink::write(writer, b",\"root\":")?;
    match summary.group {
        Group::External(root) => sink::json(writer, root)?,
        _ => sink::write(writer, b"null")?,
    }
    sink::write(writer, b",\"physical_rows\":")?;
    sink::json(writer, &summary.rows)?;
    sink::write(writer, b",\"encoded_bytes\":")?;
    sink::json(writer, &summary.bytes)?;
    sink::write(writer, b",\"rows\":[")
}
