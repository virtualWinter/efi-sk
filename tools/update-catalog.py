#!/usr/bin/env python3
"""Generate the public catalog from upstream SHA-256 lists."""

import json
import re
from functools import lru_cache
from pathlib import Path
from urllib.parse import urljoin
from urllib.request import urlopen

ROOT = Path(__file__).resolve().parents[1]


@lru_cache
def checksums(url):
    with urlopen(url, timeout=30) as response:
        data = response.read(1024 * 1024 + 1)
    if len(data) > 1024 * 1024:
        raise ValueError(f"Checksum list is too large: {url}")
    for_line = re.compile(r"([0-9a-fA-F]{64})\s+\*?([A-Za-z0-9._+-]+)")
    return [match.groups() for line in data.decode().splitlines()
            if (match := for_line.fullmatch(line.strip()))]


def resolve(source):
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
    return {
        "name": source["name"],
        "category": source["category"],
        "architecture": "x86_64",
        "filename": filename,
        "url": urljoin(source["base_url"], filename),
        "sha256": digest.lower(),
        "checksum_url": checksum_url,
    }


if __name__ == "__main__":
    sources = json.loads((ROOT / "catalog/sources.json").read_text())
    print(json.dumps({"schema_version": 1, "images": [resolve(s) for s in sources]}, indent=2))
