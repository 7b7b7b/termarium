#!/usr/bin/env python3
"""Build Termarium's compact offline land-polygon dataset.

Source data:
  Natural Earth 1:110m land GeoJSON
  https://github.com/nvkelso/natural-earth-vector/blob/master/geojson/ne_110m_land.geojson

Usage:
  python3 tools/build_world_land.py /path/to/ne_110m_land.geojson data/world_110m_land.csv
"""

from __future__ import annotations

import json
import sys
from pathlib import Path


HEADER = """# Termarium compact world land data
# Source: Natural Earth 1:110m land polygons
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


def iter_exterior_rings(geometry: dict[str, object]) -> list[list[tuple[float, float]]]:
    kind = geometry.get("type")
    coordinates = geometry.get("coordinates")
    if kind == "Polygon" and isinstance(coordinates, list):
        return exterior_rings([coordinates])
    if kind == "MultiPolygon" and isinstance(coordinates, list):
        return exterior_rings(coordinates)
    return []


def exterior_rings(polygons: list[object]) -> list[list[tuple[float, float]]]:
    rings: list[list[tuple[float, float]]] = []
    for polygon in polygons:
        if not isinstance(polygon, list) or not polygon:
            continue
        raw_ring = polygon[0]
        if not isinstance(raw_ring, list):
            continue
        ring = [point for raw_point in raw_ring if (point := valid_point(raw_point))]
        if len(ring) >= 4:
            rings.append(ring)
    return rings


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

    rings: list[list[tuple[float, float]]] = []
    for feature in data.get("features", []):
        geometry = feature.get("geometry", {}) if isinstance(feature, dict) else {}
        if not isinstance(geometry, dict):
            continue
        rings.extend(iter_exterior_rings(geometry))

    with output.open("w", encoding="utf-8", newline="") as raw:
        raw.write(HEADER)
        for ring in rings:
            raw.write(";".join(format_point(point) for point in ring))
            raw.write("\n")

    points = sum(len(ring) for ring in rings)
    print(f"land rings: {len(rings)}")
    print(f"land points: {points}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
