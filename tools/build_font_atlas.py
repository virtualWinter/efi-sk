#!/usr/bin/env python3
"""Regenerate the EFI menu's Geist glyphs with Pillow."""

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


def build_atlas(name, font_weight, size, tracking):
    font = ImageFont.truetype(
        ASSETS / f"Geist-{font_weight}.ttf", size, layout_engine=ImageFont.Layout.BASIC
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
    for face in FACES:
        build_atlas(*face)
