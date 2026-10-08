use proc_macro2::Span;
use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use syn::{Attribute, Meta, Token};

pub(super) fn test_only(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| {
        attr.path().is_ident("cfg")
            && match &attr.meta {
                Meta::List(list) => {
                    syn::parse2::<Meta>(list.tokens.clone()).is_ok_and(|meta| !possible(&meta).0)
                }
                _ => false,
            }
    })
}

fn possible(meta: &Meta) -> (bool, bool) {
    match meta {
        Meta::Path(path) if path.is_ident("test") => (false, true),
        Meta::List(list) => predicates(list),
        _ => (true, true),
    }
}

fn predicates(list: &syn::MetaList) -> (bool, bool) {
    let parser = Punctuated::<Meta, Token![,]>::parse_terminated;
    let Ok(values) = parser.parse2(list.tokens.clone()) else {
        return (true, true);
    };
    if list.path.is_ident("all") {
        return (
            values.iter().all(|meta| possible(meta).0),
            values.iter().any(|meta| possible(meta).1),
        );
    }
    if list.path.is_ident("any") {
        return (
            values.iter().any(|meta| possible(meta).0),
            values.iter().all(|meta| possible(meta).1),
        );
    }
    if list.path.is_ident("not") && values.len() == 1 {
        return values.first().map_or((true, true), |meta| {
            let (yes, no) = possible(meta);
            (no, yes)
        });
    }
    (true, true)
}

pub(super) fn item_attrs(item: &syn::Item) -> &[Attribute] {
    match item {
        syn::Item::Const(item) => &item.attrs,
        syn::Item::Enum(item) => &item.attrs,
        syn::Item::ExternCrate(item) => &item.attrs,
        syn::Item::Fn(item) => &item.attrs,
        syn::Item::ForeignMod(item) => &item.attrs,
        syn::Item::Impl(item) => &item.attrs,
        syn::Item::Macro(item) => &item.attrs,
        syn::Item::Mod(item) => &item.attrs,
        syn::Item::Static(item) => &item.attrs,
        syn::Item::Struct(item) => &item.attrs,
        syn::Item::Trait(item) => &item.attrs,
        syn::Item::TraitAlias(item) => &item.attrs,
        syn::Item::Type(item) => &item.attrs,
        syn::Item::Union(item) => &item.attrs,
        syn::Item::Use(item) => &item.attrs,
        _ => &[],
    }
}

pub(super) fn test_function(attrs: &[Attribute]) -> bool {
    test_only(attrs)
        || attrs.iter().any(|attr| {
            let path = attr.path();
            path.is_ident("test")
                || (path.segments.len() == 2
                    && path
                        .segments
                        .first()
                        .is_some_and(|part| part.ident == "tokio")
                    && path
                        .segments
                        .last()
                        .is_some_and(|part| part.ident == "test"))
        })
}

pub(super) fn item_excluded(item: &syn::Item) -> bool {
    test_only(item_attrs(item))
        || matches!(item, syn::Item::Fn(function) if test_function(&function.attrs))
}

pub(super) fn impl_attrs(item: &syn::ImplItem) -> &[Attribute] {
    match item {
        syn::ImplItem::Const(item) => &item.attrs,
        syn::ImplItem::Fn(item) => &item.attrs,
        syn::ImplItem::Type(item) => &item.attrs,
        syn::ImplItem::Macro(item) => &item.attrs,
        _ => &[],
    }
}

pub(super) fn trait_attrs(item: &syn::TraitItem) -> &[Attribute] {
    match item {
        syn::TraitItem::Const(item) => &item.attrs,
        syn::TraitItem::Fn(item) => &item.attrs,
        syn::TraitItem::Type(item) => &item.attrs,
        syn::TraitItem::Macro(item) => &item.attrs,
        _ => &[],
    }
}

pub(super) fn foreign_attrs(item: &syn::ForeignItem) -> &[Attribute] {
    match item {
        syn::ForeignItem::Fn(item) => &item.attrs,
        syn::ForeignItem::Static(item) => &item.attrs,
        syn::ForeignItem::Type(item) => &item.attrs,
        syn::ForeignItem::Macro(item) => &item.attrs,
        _ => &[],
    }
}

#[derive(Default)]
pub(super) struct Exclusions(pub(super) Vec<Span>);

impl<'ast> Visit<'ast> for Exclusions {
    fn visit_item(&mut self, item: &'ast syn::Item) {
        if item_excluded(item) {
            self.0.push(item.span());
        } else {
            visit::visit_item(self, item);
        }
    }

    fn visit_impl_item(&mut self, item: &'ast syn::ImplItem) {
        if test_only(impl_attrs(item)) {
            self.0.push(item.span());
        } else {
            visit::visit_impl_item(self, item);
        }
    }

    fn visit_trait_item(&mut self, item: &'ast syn::TraitItem) {
        if test_only(trait_attrs(item)) {
            self.0.push(item.span());
        } else {
            visit::visit_trait_item(self, item);
        }
    }

    fn visit_foreign_item(&mut self, item: &'ast syn::ForeignItem) {
        if test_only(foreign_attrs(item)) {
            self.0.push(item.span());
        } else {
            visit::visit_foreign_item(self, item);
        }
    }

    fn visit_local(&mut self, local: &'ast syn::Local) {
        if test_only(&local.attrs) {
            self.0.push(local.span());
        } else {
            visit::visit_local(self, local);
        }
    }

    fn visit_expr(&mut self, expression: &'ast syn::Expr) {
        if test_only(super::attributes::expression(expression)) {
            self.0.push(expression.span());
        } else {
            visit::visit_expr(self, expression);
        }
    }

    fn visit_arm(&mut self, arm: &'ast syn::Arm) {
        if test_only(&arm.attrs) {
            self.0.push(arm.span());
        } else {
            visit::visit_arm(self, arm);
        }
    }

    fn visit_field_value(&mut self, field: &'ast syn::FieldValue) {
        if test_only(&field.attrs) {
            self.0.push(field.span());
        } else {
            visit::visit_field_value(self, field);
        }
    }
}
