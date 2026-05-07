#!/usr/bin/env python3
"""Build Termarium's compact offline world-outline dataset.

Source data:
  Natural Earth 1:110m coastline GeoJSON
  https://github.com/nvkelso/natural-earth-vector/blob/master/geojson/ne_110m_coastline.geojson

Usage:
  python3 tools/build_world_map.py /path/to/ne_110m_coastline.geojson data/world_110m_lines.csv
"""

from __future__ import annotations

import json
import sys
from pathlib import Path


HEADER = """# Termarium compact world outline data
# Source: Natural Earth 1:110m coastline
# Natural Earth data is public domain: https://www.naturalearthdata.com/about/terms-of-use/
# Format: lon,lat;lon,lat;...
"""


def valid_point(point: object) -> tuple[float, float] | None:
    if not isinstance(point, (list, tuple)) or len(point) < 2:
        return None
    try:
        lon = float(point[0])
        lat = float(point[1])
    except (TypeError, ValueError):
        return None
    if not (-180.0 <= lon <= 180.0 and -90.0 <= lat <= 90.0):
        return None
    return lon, lat


def iter_lines(geometry: dict[str, object]) -> list[list[tuple[float, float]]]:
    kind = geometry.get("type")
    coordinates = geometry.get("coordinates")
    if kind == "LineString":
        return [clean_line(coordinates)]
    if kind == "MultiLineString" and isinstance(coordinates, list):
        return [clean_line(line) for line in coordinates]
    return []


def clean_line(raw_line: object) -> list[tuple[float, float]]:
    if not isinstance(raw_line, list):
        return []
    cleaned: list[tuple[float, float]] = []
    for raw_point in raw_line:
        point = valid_point(raw_point)
        if point is None:
            continue
        if cleaned and abs(point[0] - cleaned[-1][0]) > 180.0:
            # Avoid drawing a false whole-world segment across the dateline.
            break
        cleaned.append(point)
    return cleaned


def format_point(point: tuple[float, float]) -> str:
    lon, lat = point
    return f"{lon:.4f},{lat:.4f}"


def main() -> int:
    if len(sys.argv) != 3:
        print(__doc__.strip(), file=sys.stderr)
        return 2

    source = Path(sys.argv[1])
    output = Path(sys.argv[2])
    output.parent.mkdir(parents=True, exist_ok=True)

    with source.open(encoding="utf-8") as raw:
        data = json.load(raw)

    lines: list[list[tuple[float, float]]] = []
    for feature in data.get("features", []):
        geometry = feature.get("geometry", {}) if isinstance(feature, dict) else {}
        if not isinstance(geometry, dict):
            continue
        for line in iter_lines(geometry):
            if len(line) >= 2:
                lines.append(line)

    with output.open("w", encoding="utf-8", newline="") as raw:
        raw.write(HEADER)
        for line in lines:
            raw.write(";".join(format_point(point) for point in line))
            raw.write("\n")

    points = sum(len(line) for line in lines)
    print(f"world lines: {len(lines)}")
    print(f"world points: {points}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
