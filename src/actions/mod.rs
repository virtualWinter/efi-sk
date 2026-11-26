use alloc::{format, string::String, vec::Vec};
use uefi::proto::device_path::{DevicePath, DeviceSubType};
use uefi::proto::media::fs::SimpleFileSystem;
use uefi::runtime::{self, ResetType, VariableAttributes, VariableVendor};
use uefi::{CStr16, CString16, Handle, Status, boot, cstr16};

use crate::entries::{Entry, Target};

const GLOBAL: &VariableVendor = &VariableVendor::GLOBAL_VARIABLE;
const UI: u64 = 1;
const ATTRIBUTES: VariableAttributes = VariableAttributes::NON_VOLATILE
    .union(VariableAttributes::BOOTSERVICE_ACCESS)
    .union(VariableAttributes::RUNTIME_ACCESS);

#[derive(Clone, Copy)]
pub enum Action {
    Local(u16),
    Firmware,
    Exit,
    Reboot,
    Shutdown,
}

pub fn add(esp: Handle, entries: &mut Vec<Entry>) {
    let mut push = |label: &str, action| {
        entries.push(Entry {
            label: String::from(label),
            target: Target::Action(action),
            icon: None,
        });
    };
    if let Some(option) = local_option(esp) {
        push("Boot local drive", Action::Local(option));
    }
    if indications(cstr16!("OsIndicationsSupported")).is_ok_and(|bits| bits & UI != 0) {
        push("Firmware setup", Action::Firmware);
    }
    push("Exit to firmware", Action::Exit);
    push("Reboot", Action::Reboot);
    push("Shut down", Action::Shutdown);
}

impl Action {
    pub fn run(self) -> Result<(), Status> {
        match self {
            Self::Exit => return Ok(()),
            Self::Shutdown => runtime::reset(ResetType::SHUTDOWN, Status::SUCCESS, None),
            Self::Local(option) => {
                runtime::set_variable(
                    cstr16!("BootNext"),
                    GLOBAL,
                    ATTRIBUTES,
                    &option.to_le_bytes(),
                )
                .map_err(|error| error.status())?;
            }
            Self::Firmware => {
                let name = cstr16!("OsIndications");
                let current = match indications(name) {
                    Ok(bits) => bits,
                    Err(Status::NOT_FOUND) => 0,
                    Err(error) => return Err(error),
                };
                runtime::set_variable(name, GLOBAL, ATTRIBUTES, &(current | UI).to_le_bytes())
                    .map_err(|error| error.status())?;
            }
            Self::Reboot => {}
        }
        runtime::reset(ResetType::COLD, Status::SUCCESS, None)
    }
}

fn indications(name: &CStr16) -> Result<u64, Status> {
    let mut bytes = [0; 8];
    let (data, _) = runtime::get_variable(name, GLOBAL, &mut bytes).map_err(|err| err.status())?;
    Ok(u64::from_le_bytes(
        data.try_into().map_err(|_| Status::COMPROMISED_DATA)?,
    ))
}

fn local_option(esp: Handle) -> Option<u16> {
    let (order, _) = runtime::get_variable_boxed(cstr16!("BootOrder"), GLOBAL).ok()?;
    let mut current = [0; 2];
    let current = runtime::get_variable(cstr16!("BootCurrent"), GLOBAL, &mut current)
        .ok()
        .and_then(|(bytes, _)| bytes.try_into().ok())
        .map(u16::from_le_bytes);
    let handles = boot::find_handles::<SimpleFileSystem>().ok()?;
    let mut local_paths = Vec::new();
    for handle in handles.into_iter().filter(|handle| *handle != esp) {
        let Ok(path) = boot::open_protocol_exclusive::<DevicePath>(handle) else {
            continue;
        };
        if path.node_iter().any(|node| usb(node.sub_type())) {
            continue;
        }
        if path
            .node_iter()
            .any(|node| node.sub_type() == DeviceSubType::MEDIA_HARD_DRIVE)
        {
            local_paths.push(path.to_boxed());
        }
    }
    for id in order
        .as_chunks::<2>()
        .0
        .iter()
        .copied()
        .map(u16::from_le_bytes)
    {
        if Some(id) == current {
            continue;
        }
        let name = CString16::try_from(format!("Boot{id:04X}").as_str()).ok()?;
        let Ok((data, _)) = runtime::get_variable_boxed(&name, GLOBAL) else {
            continue;
        };
        let Some(path) = option_path(&data) else {
            continue;
        };
        let mut nodes = path.node_iter();
        let Some(first) = nodes.next() else { continue };
        if local_paths.iter().any(|local| {
            if first.sub_type() == DeviceSubType::MEDIA_HARD_DRIVE {
                local.node_iter().any(|node| node == first)
            } else {
                let mut option = path.node_iter();
                local.node_iter().all(|node| option.next() == Some(node))
            }
        }) {
            return Some(id);
        }
    }
    None
}

fn usb(subtype: DeviceSubType) -> bool {
    matches!(
        subtype,
        DeviceSubType::MESSAGING_USB
            | DeviceSubType::MESSAGING_USB_CLASS
            | DeviceSubType::MESSAGING_USB_WWID
    )
}

fn option_path(data: &[u8]) -> Option<&DevicePath> {
    if data.first()? & 1 == 0 {
        return None;
    }
    let length = usize::from(u16::from_le_bytes(data.get(4..6)?.try_into().ok()?));
    let description = data
        .get(6..)?
        .as_chunks::<2>()
        .0
        .iter()
        .position(|pair| *pair == [0, 0])?;
    let start = 6 + (description + 1) * 2;
    data.get(start..start.checked_add(length)?)?.try_into().ok()
}
