#![no_main]
#![no_std]

extern crate alloc;

mod actions;
mod chainload;
mod entries;
mod iso;
mod menu;
mod system_info;

use core::fmt::{Arguments, Write};
use entries::{ACPIDUMP, SCT_INSTALLER, Target};
use uefi::{Status, boot, entry, system};

#[entry]
fn main() -> Status {
    match run() {
        Ok(()) => Status::SUCCESS,
        Err(status) => {
            print(format_args!("Loader stopped: {status:?}\n"));
            status
        }
    }
}

fn run() -> Result<(), Status> {
    let image = boot::image_handle();
    let device = chainload::boot_volume(image)?;
    let mut entries = alloc::vec::Vec::new();
    iso::discovery::add(device, &mut entries);
    entries::add_tools(image, &mut entries);
    actions::add(device, &mut entries);

    let graphics_available =
        boot::get_handle_for_protocol::<uefi::proto::console::gop::GraphicsOutput>().is_ok();
    iso::artwork::load(device, &mut entries, graphics_available);

    let mut selected = 0;
    let mut last_boot = None;
    while menu::select(&entries, &mut selected, last_boot)? {
        let entry = &entries[selected];
        if matches!(entry.target, Target::Action(actions::Action::Exit)) {
            break;
        }
        print(format_args!("Starting {}...\n", entry.label));
        last_boot = Some(match &entry.target {
            Target::Efi(_) if entry.is_keytool() && !system_info::has_secure_boot_setup() => {
                Err(Status::UNSUPPORTED)
            }
            Target::Efi(path) if path.eq_ignore_ascii_case(SCT_INSTALLER) => {
                chainload::launch_with_options(
                    image,
                    device,
                    "\\EFI\\BOOT\\shell.efi",
                    Some("shell.efi -nostartup \\SctPackageX64\\InstallX64.efi"),
                )
            }
            Target::Efi(path) if path.eq_ignore_ascii_case(ACPIDUMP) => {
                let result = chainload::launch_with_options(
                    image,
                    device,
                    path,
                    Some("acpidump.efi -o acpidump.txt"),
                );
                if result.is_ok() {
                    print(format_args!(
                        "AcpiDump returned. Check acpidump.txt on the EFI partition.\nPress any key to return.\n"
                    ));
                    system_info::wait_for_key()?;
                    last_boot = None;
                    continue;
                }
                result
            }
            Target::Efi(path) => chainload::launch(image, device, path),
            Target::Iso(profile, path) => iso::launch(image, device, *profile, path),
            Target::Action(action) => action.run(),
            Target::SystemInfo => {
                system_info::show()?;
                last_boot = None;
                continue;
            }
        });
    }
    Ok(())
}

fn print(args: Arguments<'_>) {
    system::with_stdout(|output| {
        let _ = output.write_fmt(args);
    });
}
