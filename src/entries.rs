use alloc::{boxed::Box, string::String, vec::Vec};
use uefi::boot;
use uefi::fs::FileSystem;
use uefi::{CStr16, Handle, cstr16};

use crate::iso::{Profile, artwork::Icon};

pub const MAX_ENTRIES: usize = 9;
pub const SCT_INSTALLER: &str = "\\SctPackageX64\\InstallX64.efi";
pub const ACPIDUMP: &str = "\\EFI\\BOOT\\acpidump.efi";
pub const IPXE: &str = "\\EFI\\BOOT\\ipxe.efi";
const TOOLS: [(&str, &CStr16, &str, &str); 7] = [
    (
        ACPIDUMP,
        cstr16!("\\EFI\\BOOT\\acpidump.efi"),
        "Save ACPI Report",
        "AcpiDump",
    ),
    (
        IPXE,
        cstr16!("\\EFI\\BOOT\\ipxe.efi"),
        "Network Boot",
        "iPXE",
    ),
    (
        "\\EFI\\BOOT\\memtest86plus.efi",
        cstr16!("\\EFI\\BOOT\\memtest86plus.efi"),
        "Memory Test",
        "Memtest86+",
    ),
    (
        "\\EFI\\BOOT\\shell.efi",
        cstr16!("\\EFI\\BOOT\\shell.efi"),
        "UEFI Shell",
        "EDK II",
    ),
    (
        "\\EFI\\BOOT\\KeyTool.efi",
        cstr16!("\\EFI\\BOOT\\KeyTool.efi"),
        "Secure Boot Keys",
        "KeyTool",
    ),
    (
        SCT_INSTALLER,
        cstr16!("\\SctPackageX64\\InstallX64.efi"),
        "UEFI SCT Installer",
        "EDK II",
    ),
    (
        "\\EFI\\BOOT\\gdisk_x64.efi",
        cstr16!("\\EFI\\BOOT\\gdisk_x64.efi"),
        "Partition Editor",
        "GPT fdisk",
    ),
];

pub fn add_tools(image: Handle, entries: &mut Vec<Entry>) {
    entries.push(Entry {
        label: String::from("System Info"),
        target: Target::SystemInfo,
        icon: None,
    });
    let Ok(volume) = boot::get_image_file_system(image) else {
        return;
    };
    let mut fs = FileSystem::new(volume);
    for (target, path, label, _) in TOOLS {
        if fs.try_exists(path) == Ok(true) {
            entries.push(Entry {
                label: String::from(label),
                target: Target::Efi(String::from(target)),
                icon: None,
            });
        }
    }
}

pub struct Entry {
    pub label: String,
    pub target: Target,
    pub icon: Option<Box<Icon>>,
}

impl Entry {
    pub fn is_keytool(&self) -> bool {
        matches!(&self.target, Target::Efi(path) if path.eq_ignore_ascii_case("\\EFI\\BOOT\\KeyTool.efi"))
    }

    pub fn is_tool(&self) -> bool {
        self.tool_detail().is_some()
    }

    pub fn tool_detail(&self) -> Option<&'static str> {
        if matches!(self.target, Target::SystemInfo) {
            return Some("Read-only");
        }
        let Target::Efi(path) = &self.target else {
            return None;
        };
        TOOLS
            .iter()
            .find_map(|(target, _, _, detail)| path.eq_ignore_ascii_case(target).then_some(*detail))
    }
}

pub enum Target {
    Efi(String),
    Iso(Profile, String),
    Action(crate::actions::Action),
    SystemInfo,
}

pub fn valid_iso_path(path: &str) -> bool {
    let Some(relative) = path.strip_prefix("/iso/") else {
        return false;
    };
    path.is_ascii()
        && path.len() <= 240
        && path
            .as_bytes()
            .get(path.len().saturating_sub(4)..)
            .is_some_and(|extension| extension.eq_ignore_ascii_case(b".iso"))
        && relative.split('/').all(|part| {
            !part.is_empty()
                && part != "."
                && part != ".."
                && part
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
        })
}
