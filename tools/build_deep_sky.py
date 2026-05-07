#!/usr/bin/env python3
"""Build Termarium's small offline deep-sky catalog from OpenNGC.

Usage:
  python3 tools/build_deep_sky.py /path/to/OpenNGC/database_files/NGC.csv data/deep_sky.csv
"""

from __future__ import annotations

import csv
import sys
from pathlib import Path


FIELDS = ["name", "kind", "ra", "dec", "mag", "con", "common"]

MANUAL_MESSIER = [
    {
        "name": "M40",
        "kind": "Double Star",
        "ra": "12.3700",
        "dec": "58.0833",
        "mag": "8.4",
        "con": "UMa",
        "common": "Winnecke 4",
    },
    {
        "name": "M45",
        "kind": "Open Cluster",
        "ra": "3.7900",
        "dec": "24.1167",
        "mag": "1.6",
        "con": "Tau",
        "common": "Pleiades",
    },
    {
        "name": "M102",
        "kind": "Galaxy",
        "ra": "15.1082",
        "dec": "55.7633",
        "mag": "10.7",
        "con": "Dra",
        "common": "Spindle Galaxy",
    },
]

TYPE_NAMES = {
    "G": "Galaxy",
    "GCl": "Globular Cluster",
    "OCl": "Open Cluster",
    "PN": "Planetary Nebula",
    "Neb": "Nebula",
    "*Ass": "Star Cloud",
    "SNR": "Supernova Remnant",
    "Cl+N": "Cluster + Nebula",
    "HII": "H II Region",
    "RfN": "Reflection Nebula",
    "Other": "Deep Sky",
}

CONSTELLATION_ALIASES = {
    "Se1": "Ser",
    "Se2": "Ser",
}


def clean(value: str) -> str:
    return (value or "").replace(",", " ").replace("|", " ").strip()


def parse_ra(value: str) -> float:
    hours, minutes, seconds = value.split(":")
    return int(hours) + int(minutes) / 60.0 + float(seconds) / 3600.0


def parse_dec(value: str) -> float:
    sign = -1.0 if value.startswith("-") else 1.0
    raw = value[1:] if value[0] in "+-" else value
    degrees, minutes, seconds = raw.split(":")
    return sign * (int(degrees) + int(minutes) / 60.0 + float(seconds) / 3600.0)


def magnitude(row: dict[str, str]) -> str:
    for key in ("V-Mag", "B-Mag"):
        value = row.get(key, "").strip()
        if value:
            return f"{float(value):.2f}"
    return ""


def messier_name(value: str) -> str:
    return f"M{int(value)}"


def main() -> int:
    if len(sys.argv) != 3:
        print(__doc__.strip(), file=sys.stderr)
        return 2

    source = Path(sys.argv[1])
    output = Path(sys.argv[2])
    output.parent.mkdir(parents=True, exist_ok=True)

    rows: list[dict[str, str]] = []
    seen: set[str] = set()
    with source.open(newline="", encoding="utf-8") as raw:
        reader = csv.DictReader(raw, delimiter=";")
        for row in reader:
            messier = row.get("M", "").strip()
            if not messier:
                continue
            name = messier_name(messier)
            seen.add(name)
            common = clean(row.get("Common names", ""))
            if common:
                common = common.split(";")[0].strip()
            rows.append(
                {
                    "name": name,
                    "kind": TYPE_NAMES.get(row.get("Type", ""), "Deep Sky"),
                    "ra": f"{parse_ra(row['RA']):.6f}",
                    "dec": f"{parse_dec(row['Dec']):.6f}",
                    "mag": magnitude(row),
                    "con": CONSTELLATION_ALIASES.get(clean(row.get("Const", "")), clean(row.get("Const", ""))),
                    "common": common,
                }
            )

    for row in MANUAL_MESSIER:
        if row["name"] not in seen:
            rows.append(row)

    rows.sort(key=lambda item: int(item["name"][1:]))

    with output.open("w", newline="", encoding="utf-8") as raw:
        writer = csv.DictWriter(raw, fieldnames=FIELDS)
        writer.writeheader()
        writer.writerows(rows)

    print(f"deep-sky objects: {len(rows)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
