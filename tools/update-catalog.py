#!/usr/bin/env python3
"""Generate the public image catalog and distro artwork index."""

import json
import re
from functools import lru_cache
from pathlib import Path
from typing import TypedDict
from urllib.parse import urljoin
from urllib.request import urlopen

ROOT = Path(__file__).resolve().parents[1]


@lru_cache
def checksums(url):
    with urlopen(url, timeout=30) as response:
        data = response.read(1024 * 1024 + 1)
    if len(data) > 1024 * 1024:
        raise ValueError(f"Checksum list is too large: {url}")
    gnu = re.compile(r"([0-9a-fA-F]{64})\s+\*?([A-Za-z0-9._+-]+)")
    bsd = re.compile(r"SHA256 \(([A-Za-z0-9._+-]+)\) = ([0-9a-fA-F]{64})")
    result = set()
    for line in data.decode().splitlines():
        if match := gnu.fullmatch(line.strip()):
            digest, filename = match.groups()
        elif match := bsd.fullmatch(line.strip()):
            filename, digest = match.groups()
        else:
            continue
        result.add((digest.lower(), filename))
    return result


def resolve(source):
    if "sha256" in source:
        filename, digest = source["filename"], source["sha256"]
        if not re.fullmatch(r"[A-Za-z0-9._+ -]+", filename) or not re.fullmatch(r"[a-fA-F0-9]{64}", digest):
            raise ValueError(f"Invalid pinned image: {source['name']}")
        return image_entry(source, filename, digest, urljoin(source["base_url"], source["checksum_url"]))

    checksum_url = urljoin(source["base_url"], source["checksum_file"])
    pattern = source.get("filename_pattern") or re.escape(source["filename"])
    matches = [(digest, name) for digest, name in checksums(checksum_url)
               if re.fullmatch(pattern, name)]
    if "filename_pattern" in source and matches:
        def version(match):
            return tuple(map(int, re.findall(r"\d+", match[1])))

        newest = max(map(version, matches))
        matches = [match for match in matches if version(match) == newest]
    if len(matches) != 1:
        raise ValueError(f"Expected one match for {source['name']} in {checksum_url}, got {len(matches)}")
    digest, filename = matches[0]
    return image_entry(source, filename, digest, checksum_url)


def image_entry(source, filename, digest, checksum_url):
    return {
        "distro": source["distro"],
        "name": source["name"],
        "category": source["category"],
        "architecture": "x86_64",
        "filename": filename,
        "url": urljoin(source["base_url"], filename),
        "sha256": digest.lower(),
        "checksum_url": checksum_url,
        "boot_status": source.get("boot_status", "untested"),
    }


class Distro(TypedDict):
    id: str
    rank: int | None
    icon: str
    artwork: str
    aliases: list[str]


def distro_index() -> list[Distro]:
    distros: dict[str, Distro] = {}
    for line in (ROOT / "assets/distro/index.tsv").read_text().splitlines():
        if line.startswith("#"):
            continue
        rank, slug, icon, kind, aliases = line.split("\t")
        row = distros.setdefault(slug, {
            "id": slug,
            "rank": int(rank) if rank != "-" else None,
            "icon": icon,
            "artwork": kind,
            "aliases": [],
        })
        row["aliases"].extend(aliases.split(","))
    return list(distros.values())


def download_index(distros, images, pages):
    by_distro: dict[str, list[str]] = {}
    for image in images:
        by_distro.setdefault(image["distro"], []).append(image["url"])
    downloads = []
    for distro in distros:
        slug = distro["id"]
        page = pages.get(slug)
        if page is None and distro["rank"] is not None:
            page = f"https://distrowatch.com/{slug}"
        if not page or not page.startswith("https://"):
            raise ValueError(f"Missing HTTPS download reference for {slug}")
        urls = by_distro.get(slug, [])
        downloads.append({
            "distro": slug,
            "page": page,
            "status": "verified-images" if urls else "reference-only",
            "image_urls": urls,
        })
    return downloads


if __name__ == "__main__":
    sources = json.loads((ROOT / "catalog/sources.json").read_text())
    distros = distro_index()
    known = {distro["id"] for distro in distros}
    for source in sources:
        if source["distro"] not in known:
            raise ValueError(f"Unknown artwork entry: {source['distro']}")
    images = [resolve(source) for source in sources]
    pages = json.loads((ROOT / "catalog/download-pages.json").read_text())
    if unknown := pages.keys() - known:
        raise ValueError(f"Download references without artwork entries: {sorted(unknown)}")
    print(json.dumps({
        "schema_version": 1,
        "distros": distros,
        "images": images,
        "downloads": download_index(distros, images, pages),
    }, indent=2))
