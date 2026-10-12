use super::xml::{Element, Handler};
use super::{Archive, Scope, WORKBOOK, WORKBOOK_RELS};
use crate::report::ReportResult;
use std::collections::BTreeMap;

pub(super) struct Sheet {
    pub(super) name: String,
    pub(super) member: String,
}

pub(super) fn worksheets(archive: &Archive) -> ReportResult<Vec<Sheet>> {
    let refs = archive.read_member(WORKBOOK, References::new(archive.scope(WORKBOOK)))?;
    let rels = archive.read_member(
        WORKBOOK_RELS,
        Relationships::new(archive.scope(WORKBOOK_RELS)),
    )?;
    let scope = archive.scope(WORKBOOK);
    let mut sheets = Vec::new();
    sheets
        .try_reserve(refs.items.len())
        .map_err(|_| scope.corrupt("the worksheet table cannot be allocated"))?;
    for (name, id) in refs.items {
        let target = rels
            .targets
            .get(&id)
            .ok_or_else(|| scope.corrupt(format!("worksheet {name} refers to a missing {id}")))?;
        sheets.push(Sheet {
            name,
            member: member(target, &scope)?,
        });
    }
    Ok(sheets)
}

fn member(target: &str, scope: &Scope<'_>) -> ReportResult<String> {
    if target.is_empty()
        || target.starts_with('/')
        || target.contains("..")
        || target.contains('\\')
        || target.contains(':')
    {
        return Err(scope.corrupt(format!("worksheet target {target} escapes the package")));
    }
    Ok(format!("xl/{target}"))
}

struct References<'a> {
    scope: Scope<'a>,
    items: Vec<(String, String)>,
}

impl<'a> References<'a> {
    fn new(scope: Scope<'a>) -> Self {
        Self {
            scope,
            items: Vec::new(),
        }
    }
}

impl Handler for References<'_> {
    fn start(&mut self, element: &Element<'_>) -> ReportResult<()> {
        if element.name != "sheet" {
            return Ok(());
        }
        let name = element.required("name", &self.scope)?.to_string();
        let id = element
            .attribute("r:id")
            .or_else(|| element.attribute("id"))
            .ok_or_else(|| {
                self.scope
                    .corrupt("a worksheet element lacks its relationship id")
            })?
            .to_string();
        self.items.push((name, id));
        Ok(())
    }

    fn end(&mut self, _name: &str) -> ReportResult<()> {
        Ok(())
    }

    fn text(&mut self, _text: &str) -> ReportResult<()> {
        Ok(())
    }
}

struct Relationships<'a> {
    scope: Scope<'a>,
    targets: BTreeMap<String, String>,
}

impl<'a> Relationships<'a> {
    fn new(scope: Scope<'a>) -> Self {
        Self {
            scope,
            targets: BTreeMap::new(),
        }
    }
}

impl Handler for Relationships<'_> {
    fn start(&mut self, element: &Element<'_>) -> ReportResult<()> {
        if element.name != "Relationship" {
            return Ok(());
        }
        let kind = element.attribute("Type").map_or("", |kind| kind);
        let target = element.required("Target", &self.scope)?.to_string();
        if !kind.ends_with("/worksheet") && !target.starts_with("worksheets/") {
            return Ok(());
        }
        let id = element.required("Id", &self.scope)?.to_string();
        self.targets.insert(id, target);
        Ok(())
    }

    fn end(&mut self, _name: &str) -> ReportResult<()> {
        Ok(())
    }

    fn text(&mut self, _text: &str) -> ReportResult<()> {
        Ok(())
    }
}
