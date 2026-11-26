#!/usr/bin/env python3
"""Boot a physical USB disk read-only in QEMU using UDisks2."""

import argparse
import os
from pathlib import Path
import subprocess

from gi.repository import Gio, GLib


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("device", type=Path, help="whole USB disk, e.g. /dev/sdb")
    args = parser.parse_args()
    disk = args.device.resolve(strict=True)
    identity = subprocess.check_output(
        ["lsblk", "-dnro", "TYPE,TRAN", "--", disk], text=True
    ).split()
    if identity != ["disk", "usb"]:
        parser.error(f"{disk} is not a whole USB disk")
    firmware = os.environ.get("OVMF_FIRMWARE") or next(
        (str(p) for p in map(Path, (
            "/usr/share/OVMF/x64/OVMF.4m.fd",
            "/usr/share/edk2/x64/OVMF.4m.fd",
            "/usr/share/OVMF/OVMF_CODE_4M.fd",
        )) if p.is_file()), None
    )
    if not firmware:
        parser.error("set OVMF_FIRMWARE to your x86-64 OVMF firmware file")
    bus = Gio.bus_get_sync(Gio.BusType.SYSTEM, None)
    reply, fds = bus.call_with_unix_fd_list_sync(
        "org.freedesktop.UDisks2", f"/org/freedesktop/UDisks2/block_devices/{disk.name}",
        "org.freedesktop.UDisks2.Block", "OpenDevice",
        GLib.Variant("(sa{sv})", ("r", {})), None, Gio.DBusCallFlags.NONE,
        -1, None, None,
    )
    fd = fds.get(reply.unpack()[0])
    try:
        print(f"Booting {disk} read-only. Close the QEMU window to exit.", flush=True)
        result = subprocess.run([
            "qemu-system-x86_64", "-machine", "q35,accel=kvm:tcg", "-m", "2G",
            "-bios", firmware, "-add-fd", f"fd={fd},set=1",
            "-blockdev", "driver=host_device,node-name=stick,filename=/dev/fdset/1,read-only=on",
            "-device", "qemu-xhci", "-device", "usb-storage,drive=stick",
            "-nic", "none", "-display", "gtk",
        ], pass_fds=(fd,))
        raise SystemExit(result.returncode)
    finally:
        os.close(fd)


if __name__ == "__main__":
    try:
        main()
    except (OSError, subprocess.CalledProcessError, GLib.Error) as error:
        raise SystemExit(str(error)) from error
