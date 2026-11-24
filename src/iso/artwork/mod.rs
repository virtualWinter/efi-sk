pub const ICON_SIZE: usize = 18;
pub type Icon = [u8; ICON_SIZE * ICON_SIZE * 4];

include!(concat!(env!("OUT_DIR"), "/distro_icons.rs"));

pub(super) fn for_image(stem: &str) -> Option<&'static Icon> {
    let mut normalized = [0; 240];
    let mut len = 0;
    for byte in stem.bytes().filter(u8::is_ascii_alphanumeric) {
        *normalized.get_mut(len)? = byte.to_ascii_lowercase();
        len += 1;
    }
    if stem
        .get(..5)
        .is_some_and(|start| start.eq_ignore_ascii_case("ARCH_"))
    {
        return Some(include_bytes!("../../../assets/distro/archlinux.rgba"));
    }
    // Check longer aliases first so e.g. Ubuntu Studio wins over Ubuntu.
    PREFIXES.iter().find_map(|&(prefix, slot)| {
        let tail = normalized[..len].strip_prefix(prefix)?;
        // Short distro names are useful, but not as arbitrary word prefixes.
        (prefix.len() > 3
            || tail.first().is_none_or(u8::is_ascii_digit)
            || starts_on_token_boundary(stem, prefix.len()))
        .then(|| ICONS[slot])
    })
}

fn starts_on_token_boundary(stem: &str, normalized_len: usize) -> bool {
    let mut chars = stem
        .bytes()
        .enumerate()
        .filter(|(_, b)| b.is_ascii_alphanumeric());
    chars.nth(normalized_len - 1).is_some_and(|(end, _)| {
        stem.as_bytes()
            .get(end + 1)
            .is_some_and(|b| !b.is_ascii_alphanumeric())
    })
}
