use alloc::string::String;

/// The display name from `os-release`: `PRETTY_NAME`, falling back to `NAME`.
pub fn pretty_name(release: &str) -> Option<String> {
    ["PRETTY_NAME=", "NAME="]
        .into_iter()
        .filter_map(|key| field(release, key))
        .find(|name| {
            !name.is_empty()
                && name.len() <= 60
                && name.bytes().all(|b| b.is_ascii_graphic() || b == b' ')
        })
        .map(String::from)
}

/// The `LOGO` value from `os-release`: a file name stem in `pixmaps/`.
pub fn logo(release: &str) -> Option<&str> {
    field(release, "LOGO=").filter(|name| {
        !name.is_empty()
            && name.len() <= 64
            && name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'))
    })
}

fn field<'a>(release: &'a str, key: &str) -> Option<&'a str> {
    let value = release
        .lines()
        .find_map(|line| line.strip_prefix(key))?
        .trim();
    for quote in ['"', '\''] {
        if let Some(value) = value.strip_prefix(quote) {
            return value.strip_suffix(quote);
        }
    }
    Some(value)
}
