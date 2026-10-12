use super::super::cache::content_digest;

pub(super) fn permits(url: &str, name: &str, value: &str) -> bool {
    let Ok(target) = url::Url::parse(url) else {
        return false;
    };
    if target.scheme() != "https"
        || target.port().is_some()
        || !target.username().is_empty()
        || target.password().is_some()
    {
        return false;
    }
    match target.host_str() {
        Some("www.cifsshome.org")
            if numbered_path(target.path(), "/widget/get-school-details/", "/details") =>
        {
            match name {
                "x-requested-with" => value == "XMLHttpRequest",
                "referer" => url::Url::parse(value).is_ok_and(|referer| {
                    referer.origin() == target.origin()
                        && referer.username().is_empty()
                        && referer.password().is_none()
                        && referer.path() == "/widget/school/directory"
                }),
                _ => false,
            }
        }
        Some("services.arbitersports.com")
            if numbered_path(target.path(), "/api/v2/organization/public/", "/children")
                || numbered_path(target.path(), "/api/v2/legacy/public/", "/coaches") =>
        {
            name == "authorization"
                && value.strip_prefix("Bearer ").is_some_and(|token| {
                    !token.is_empty() && token.bytes().all(|byte| byte.is_ascii_graphic())
                })
        }
        _ => false,
    }
}

fn numbered_path(path: &str, prefix: &str, suffix: &str) -> bool {
    path.strip_prefix(prefix)
        .and_then(|rest| rest.strip_suffix(suffix))
        .is_some_and(|id| !id.is_empty() && id.bytes().all(|byte| byte.is_ascii_digit()))
}

pub(super) fn recorded_value(name: &str, value: &str) -> String {
    if name == "authorization" {
        format!("sha256:{}", content_digest(value.as_bytes()))
    } else {
        value.to_string()
    }
}
