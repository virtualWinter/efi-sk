#!/usr/bin/env python3
"""Regenerate the EFI menu's Geist glyphs with Pillow."""

import argparse
from pathlib import Path
import struct

from PIL import ImageFont

ASSETS = Path(__file__).resolve().parents[1] / "assets"
GLYPHS = [chr(code) for code in range(32, 127)] + ["…"]
FACES = [
    ("regular", "Regular", 14, 0),
    ("semibold", "SemiBold", 22, -0.025),
]
RECORD = struct.Struct("<IHbbBB")


def build_atlas(font_dir, name, font_weight, size, tracking):
    font = ImageFont.truetype(
        font_dir / f"Geist-{font_weight}.ttf", size, layout_engine=ImageFont.Layout.BASIC
    )
    records = bytearray()
    pixels = bytearray()
    for character in GLYPHS:
        mask, (left, top) = font.getmask2(character, mode="L", anchor="ls")
        width, height = mask.size
        advance = round((font.getlength(character) + size * tracking) * 64)
        records.extend(RECORD.pack(len(pixels), advance, left, top, width, height))
        pixels.extend(bytes(mask))
    (ASSETS / f"geist-{name}.atlas").write_bytes(records + pixels)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("font_dir", type=Path, help="directory containing the Geist static TTFs")
    font_dir = parser.parse_args().font_dir
    for _name, weight, _size, _tracking in FACES:
        font = font_dir / f"Geist-{weight}.ttf"
        if not font.is_file():
            parser.error(f"missing font: {font}")
    for face in FACES:
        build_atlas(font_dir, *face)
