//! HTML element-close rules for scopes the profile parser must reject.

pub(super) fn implicitly_closes(current: &str, next: &str) -> bool {
    matches!(
        (current, next),
        (
            "p",
            "address"
                | "article"
                | "aside"
                | "blockquote"
                | "div"
                | "dl"
                | "fieldset"
                | "footer"
                | "form"
                | "h1"
                | "h2"
                | "h3"
                | "h4"
                | "h5"
                | "h6"
                | "header"
                | "hgroup"
                | "hr"
                | "main"
                | "menu"
                | "nav"
                | "ol"
                | "p"
                | "pre"
                | "section"
                | "table"
                | "ul"
        ) | ("li", "li")
            | ("dt" | "dd", "dt" | "dd")
            | ("rt" | "rp", "rt" | "rp")
            | ("option", "option" | "optgroup")
            | ("optgroup", "optgroup")
            | ("thead" | "tbody" | "tfoot", "thead" | "tbody" | "tfoot")
            | ("tr", "tr")
            | ("td" | "th", "td" | "th")
    )
}
