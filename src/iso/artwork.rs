pub const ICON_SIZE: usize = 18;
pub type Icon = [u8; ICON_SIZE * ICON_SIZE * 4];

pub(super) fn for_image(filename: &str) -> Option<&'static Icon> {
    let filename = filename.to_ascii_lowercase();
    for (prefixes, icon) in [
        (
            &["proxmox"][..],
            include_bytes!("../../assets/distro/proxmox.rgba"),
        ),
        (
            &["archlinux", "arch-linux"][..],
            include_bytes!("../../assets/distro/archlinux.rgba"),
        ),
        (
            &["debian", "lmde"][..],
            include_bytes!("../../assets/distro/debian.rgba"),
        ),
        (
            &["fedora"][..],
            include_bytes!("../../assets/distro/fedora.rgba"),
        ),
        (
            &["ubuntu", "kubuntu", "lubuntu", "xubuntu", "edubuntu"][..],
            include_bytes!("../../assets/distro/ubuntu.rgba"),
        ),
        (
            &["opensuse"][..],
            include_bytes!("../../assets/distro/opensuse.rgba"),
        ),
        (
            &["rocky"][..],
            include_bytes!("../../assets/distro/rockylinux.rgba"),
        ),
        (
            &["almalinux"][..],
            include_bytes!("../../assets/distro/almalinux.rgba"),
        ),
        (
            &["kali"][..],
            include_bytes!("../../assets/distro/kalilinux.rgba"),
        ),
    ] {
        if prefixes.iter().any(|prefix| filename.starts_with(prefix)) {
            return Some(icon);
        }
    }
    None
}
