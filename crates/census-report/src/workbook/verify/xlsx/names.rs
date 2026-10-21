use super::xml::{Element, Handler};
use super::{Archive, Scope, WORKBOOK, WORKBOOK_RELS};
use crate::report::ReportResult;
use std::collections::{btree_map::Entry, BTreeMap};

pub(super) struct Sheet {
    pub(super) name: String,
    pub(super) member: String,
}

pub(super) fn worksheets(archive: &Archive) -> ReportResult<Vec<Sheet>> {
    let refs = archive.read_member(
        WORKBOOK,
        References::new(archive.scope(WORKBOOK), archive.profile.members),
    )?;
    let rels = archive.read_member(
        WORKBOOK_RELS,
        Relationships::new(archive.scope(WORKBOOK_RELS), archive.profile.members),
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
            .and_then(Option::as_deref)
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
    limit: usize,
}

impl<'a> References<'a> {
    fn new(scope: Scope<'a>, limit: usize) -> Self {
        Self {
            scope,
            items: Vec::new(),
            limit,
        }
    }
}

impl Handler for References<'_> {
    fn start(&mut self, element: &Element<'_>) -> ReportResult<()> {
        if element.name != "sheet" {
            return Ok(());
        }
        if self.items.len() >= self.limit {
            return Err(self
                .scope
                .corrupt("the worksheet table exceeds its member budget"));
        }
        let name = element.required("name", &self.scope)?;
        if name.is_empty() {
            return Err(self.scope.corrupt("a worksheet has an empty name"));
        }
        let rid = element
            .attribute("r:id")
            .filter(|id| !id.is_empty())
            .ok_or_else(|| {
                self.scope
                    .corrupt("a worksheet element lacks its relationship id")
            })?;
        for (attr_name, _) in element.attributes {
            if attr_name == "r:id" {
                continue;
            }
            if attr_name == "id" || (attr_name.contains(':') && attr_name.ends_with(":id")) {
                return Err(self.scope.corrupt(format!(
                    "worksheet {name} has ambiguous id attribute {attr_name}"
                )));
            }
        }
        if self
            .items
            .iter()
            .any(|(prior, _)| prior.eq_ignore_ascii_case(name))
        {
            return Err(self
                .scope
                .corrupt(format!("worksheet name {name} is repeated")));
        }
        if self.items.iter().any(|(_, prior)| prior == rid) {
            return Err(self
                .scope
                .corrupt(format!("worksheet relationship {rid} is reused")));
        }
        self.items.try_reserve(1).map_err(|_| {
            self.scope
                .corrupt("the worksheet table cannot be allocated")
        })?;
        self.items.push((name.to_string(), rid.to_string()));
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
    targets: BTreeMap<String, Option<String>>,
    limit: usize,
}

impl<'a> Relationships<'a> {
    fn new(scope: Scope<'a>, limit: usize) -> Self {
        Self {
            scope,
            targets: BTreeMap::new(),
            limit,
        }
    }
}

impl Handler for Relationships<'_> {
    fn start(&mut self, element: &Element<'_>) -> ReportResult<()> {
        if element.name != "Relationship" {
            return Ok(());
        }
        if self.targets.len() >= self.limit {
            return Err(self
                .scope
                .corrupt("the relationship table exceeds its member budget"));
        }
        let kind = element.attribute("Type").map_or("", |kind| kind);
        let target = element.required("Target", &self.scope)?;
        let worksheet = kind.ends_with("/worksheet") || target.starts_with("worksheets/");
        if worksheet && !matches!(element.attribute("TargetMode"), None | Some("Internal")) {
            return Err(self
                .scope
                .corrupt("a worksheet relationship must target an internal package member"));
        }
        let target = worksheet.then(|| target.to_string());
        let id = element.required("Id", &self.scope)?.to_string();
        if id.is_empty() {
            return Err(self
                .scope
                .corrupt("a workbook relationship has an empty Id"));
        }
        match self.targets.entry(id) {
            Entry::Vacant(entry) => {
                entry.insert(target);
                Ok(())
            }
            Entry::Occupied(entry) => Err(self
                .scope
                .corrupt(format!("workbook relationship {} is repeated", entry.key()))),
        }
    }

    fn end(&mut self, _name: &str) -> ReportResult<()> {
        Ok(())
    }

    fn text(&mut self, _text: &str) -> ReportResult<()> {
        Ok(())
    }
}
