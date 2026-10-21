use super::super::{
    tests::{capture, verdict, TestResult},
    *,
};
use super::scan::Directory;

fn page(binding: &TableBindingV2, index: u32) -> TestResult<RawPageV2> {
    let first = u64::from(index)
        .checked_mul(u64::from(MAX_PAGE_ROWS))
        .ok_or("ordinal")?;
    let mut rows = reserve(usize::from(MAX_PAGE_ROWS))?;
    for offset in 0..MAX_PAGE_ROWS {
        let ordinal = first.checked_add(u64::from(offset)).ok_or("ordinal")?;
        let id = format!("verdict-{ordinal:020}");
        let key = crate::keys::observation_key(crate::Table::IdentityVerdicts, &id, 0);
        rows.push(RawPhysicalRowV2::new(
            binding,
            &key,
            &serde_json::to_vec(&verdict(&id))?,
        )?);
    }
    Ok(RawPageV2::new(
        binding,
        RawPagePositionV2::new(index, first)?,
        rows,
    )?)
}

#[test]
fn directory_flushes_256_pages_then_retains_one_page_terminal_block() -> TestResult {
    let binding = capture()?.table_binding(AuthorityTableV2::IdentityVerdicts)?;
    let mut directory = Directory::new()?;
    let mut block_sizes = Vec::new();
    let mut visit = |node: admission::Node<'_>| match node {
        admission::Node::Block(block) => {
            block_sizes.push(block.pages().len());
            let bytes = block.encode_jsonl()?;
            block.object_reference(
                u64::try_from(bytes.len()).map_err(|_| crate::StoreError::CounterOverflow)?,
            )
        }
        admission::Node::Table(table) => {
            let bytes = table.encode_jsonl()?;
            table.object_reference(
                u64::try_from(bytes.len()).map_err(|_| crate::StoreError::CounterOverflow)?,
            )
        }
        admission::Node::Page(_) | admission::Node::Seal(_) => {
            Err(invalid("unexpected directory node"))
        }
    };
    for index in 0..257 {
        let page = page(&binding, index)?;
        let bytes = page.encode_jsonl()?;
        let reference = page.reference(page.object_reference(u64::try_from(bytes.len())?)?)?;
        directory.push(&binding, reference, &mut visit)?;
    }
    let table = directory.finish(&binding, &mut visit)?;
    check!(eq;block_sizes, [256, 1]);
    check!(eq;table.inventory().pages(), 257);
    check!(eq;table.inventory().rows(), 131_584);
    check!(eq;table.inventory().object_count(), 260);
    Ok(())
}
