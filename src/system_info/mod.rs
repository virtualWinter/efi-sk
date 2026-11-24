use core::fmt::Write;
use uefi::boot::{self, MemoryType, OpenProtocolAttributes, OpenProtocolParams, ScopedProtocol};
use uefi::mem::memory_map::MemoryMap;
use uefi::proto::ProtocolPointer;
use uefi::proto::{console::gop::GraphicsOutput, media::block::BlockIO};
use uefi::runtime::{self, VariableVendor};
use uefi::{Handle, Status, cstr16, system};

fn inspect<P: ProtocolPointer + ?Sized>(handle: Handle) -> uefi::Result<ScopedProtocol<P>> {
    // Read-only queries while the firmware's existing drivers remain connected.
    // Exclusive access would disconnect drivers that own the disk or console.
    unsafe {
        boot::open_protocol(
            OpenProtocolParams {
                handle,
                agent: boot::image_handle(),
                controller: None,
            },
            OpenProtocolAttributes::GetProtocol,
        )
    }
}

pub fn has_secure_boot_setup() -> bool {
    matches!(
        runtime::get_variable(
            cstr16!("SetupMode"),
            &VariableVendor::GLOBAL_VARIABLE,
            &mut [0]
        ),
        Ok(([0 | 1], _))
    )
}

pub fn show() -> Result<(), Status> {
    let secure_boot = match runtime::get_variable(
        cstr16!("SecureBoot"),
        &VariableVendor::GLOBAL_VARIABLE,
        &mut [0],
    ) {
        Ok(([1], _)) => "Enabled",
        Ok(([0], _)) => "Disabled",
        _ => "Unavailable",
    };
    let memory = boot::memory_map(MemoryType::LOADER_DATA).ok().map(|map| {
        map.entries()
            .filter(|entry| entry.ty == MemoryType::CONVENTIONAL)
            .map(|entry| entry.page_count)
            .sum::<u64>()
            / 256
    });
    let display = boot::get_handle_for_protocol::<GraphicsOutput>()
        .ok()
        .and_then(|handle| inspect::<GraphicsOutput>(handle).ok())
        .map(|gop| gop.current_mode_info().resolution());
    let disks = boot::find_handles::<BlockIO>().ok().map(|handles| {
        handles
            .into_iter()
            .filter(|&handle| {
                inspect::<BlockIO>(handle).is_ok_and(|io| {
                    let media = io.media();
                    media.is_media_present() && !media.is_logical_partition()
                })
            })
            .count()
    });
    system::with_stdout(|out| {
        let _ = out.clear();
        let _ = writeln!(out, "\n  System Info\n");
        let _ = writeln!(
            out,
            "  Firmware: {} (revision {})",
            system::firmware_vendor(),
            system::firmware_revision()
        );
        let _ = writeln!(out, "  UEFI: {}", system::uefi_revision());
        let _ = writeln!(out, "  Secure Boot: {secure_boot}");
        if let Some(memory) = memory {
            let _ = writeln!(out, "  Available conventional memory: {memory} MiB");
        }
        if let Some((width, height)) = display {
            let _ = writeln!(out, "  Display: {width} x {height}");
        }
        if let Some(disks) = disks {
            let _ = writeln!(out, "  Detected block devices: {disks}");
        }
        let _ = writeln!(out, "\n  Press any key to return.");
    });
    wait_for_key()
}

pub fn wait_for_key() -> Result<(), Status> {
    let event =
        system::with_stdin(|input| input.wait_for_key_event()).map_err(|err| err.status())?;
    loop {
        boot::wait_for_event(core::slice::from_ref(&event)).map_err(|err| err.status())?;
        if system::with_stdin(|input| input.read_key())
            .map_err(|err| err.status())?
            .is_some()
        {
            return Ok(());
        }
    }
}
