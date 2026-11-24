# Firmware distro artwork

[`index.tsv`](index.tsv) covers the first 400 entries in the
[DistroWatch 12-month Page Hit Ranking](https://distrowatch.com/dwres.php?resource=popularity),
snapshot 2026-11-24, plus a few additional images. The ranking is used only
for coverage; logo recognition does not imply boot support.

`logo` uses the distro's own mark when available; `family` uses a related
project's mark; `generic` uses a neutral Tux icon. Unknown filenames use
the picker's generic boot-image glyph. Matching is case-insensitive,
ignores filename separators, and prefers the longest matching alias.

SVG sources come from [Simple Icons 14.15.0 and 16.38.1](https://github.com/simple-icons/simple-icons)
([CC0](../../installer/icons/LICENSE.simple-icons)) and
[font-logos at `d3bf5d2`](https://github.com/lukas-w/font-logos/tree/d3bf5d299e54595db1b19681a0cc57ab10454857)
([Unlicense](LICENSE.font-logos)). The icons from 16.38.1 are `gnome`,
`kde`, `omarchy`, `reactos`, `steam`, `talos`, and `truenas`. Existing
colored marks share their [source artwork with the installer](../../installer/icons/README.md).
Marks remain subject to their respective owners' trademark policies.

The picker embeds 18×18 RGBA renders. The build reads the checked-in
index and assets offline; it does not scrape DistroWatch at build or boot.
