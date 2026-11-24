use alloc::{format, string::String, vec::Vec};
use uefi::Handle;

use super::{Profile, filesystem::Iso, volume};
use crate::entries::{Entry, MAX_ENTRIES, Target, valid_iso_path};

pub fn add(esp: Handle, entries: &mut Vec<Entry>) {
    let mut boot_count = entries.iter().filter(|entry| !entry.is_tool()).count();
    if boot_count >= MAX_ENTRIES {
        return;
    }
    let Ok(partition) = volume::open(esp) else {
        return;
    };
    let Ok(directory) = partition.read_dir("/iso") else {
        return;
    };

    let mut paths: Vec<String> = directory
        .filter_map(Result::ok)
        .filter_map(|file| {
            if !file.file_type().ok()?.is_regular_file() {
                return None;
            }
            let name = file.file_name();
            let name = name.as_str().ok()?;
            let path = format!("/iso/{name}");
            (name.len() > 4 && valid_iso_path(&path)).then_some(path)
        })
        .collect();
    paths.sort_unstable();

    for path in paths {
        if boot_count >= MAX_ENTRIES {
            break;
        }
        let Ok(file) = partition.open(path.as_str()) else {
            continue;
        };
        let Ok(mut iso) = Iso::open(file) else {
            continue;
        };
        let Some(profile) = Profile::ALL
            .into_iter()
            .find(|profile| profile.detect(&mut iso, &partition, &path))
        else {
            continue;
        };
        entries.push(Entry {
            label: if matches!(profile, Profile::EfiMedia) {
                format!("{} (experimental EFI)", path.trim_start_matches("/iso/"))
            } else {
                String::from(profile.name())
            },
            target: Target::Iso(profile, path),
            icon: None,
        });
        boot_count += 1;
    }
}
