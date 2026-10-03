pub(super) fn table<'a>(html: &'a str, marker: &str) -> Result<&'a str, &'static str> {
    html.split("<table")
        .skip(1)
        .find_map(|candidate| {
            let (_, body) = candidate.split_once('>')?;
            let (body, _) = body.split_once("</table>")?;
            body.contains(marker).then_some(body)
        })
        .ok_or("recognized complete page table is absent")
}

pub(super) fn balanced(table: &str) -> Result<(), &'static str> {
    [
        ("<tr", "</tr>"),
        ("<td", "</td>"),
        ("<th", "</th>"),
        ("<span", "</span>"),
        ("<a ", "</a>"),
    ]
    .into_iter()
    .try_for_each(|(open, close)| {
        if table.matches(open).count() == table.matches(close).count() {
            Ok(())
        } else {
            Err("page table contains unfinished rows, cells or published fields")
        }
    })
}

pub(super) fn rows(table: &str) -> impl Iterator<Item = Result<&str, &'static str>> {
    table.split("<tr").skip(1).map(|candidate| {
        let (row, _) = candidate
            .split_once("</tr>")
            .ok_or("table row is unfinished")?;
        if row.contains("<table") {
            return Err("unexpected nested table in published row");
        }
        Ok(row)
    })
}

pub(super) fn cells<'a>(row: &'a str, tag: &str) -> Result<[Option<&'a str>; 3], &'static str> {
    let (open, close) = match tag {
        "td" => ("<td", "</td>"),
        "th" => ("<th", "</th>"),
        _ => return Err("unsupported table cell kind"),
    };
    row.split(open)
        .skip(1)
        .try_fold(([None; 3], 0_usize), |(mut cells, count), candidate| {
            let slot = cells
                .get_mut(count)
                .ok_or("published row exceeds three cells")?;
            let (_, cell) = candidate
                .split_once('>')
                .ok_or("cell opening is unfinished")?;
            let (cell, _) = cell.split_once(close).ok_or("cell value is unfinished")?;
            *slot = Some(cell);
            Ok((cells, count.saturating_add(1)))
        })
        .map(|(cells, _)| cells)
}
