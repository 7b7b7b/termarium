#!/usr/bin/env python3
"""Build Termarium's offline bright-star catalog from HYG v4.2.

Usage:
  python3 tools/build_catalog.py /path/to/hyg_v42.csv.gz data/hyg_v42_bright.csv
"""

from __future__ import annotations

import csv
import gzip
import sys
from pathlib import Path


FIELDS = ["hip", "proper", "ra", "dec", "mag", "ci", "con"]


def clean_text(value: str) -> str:
    return value.replace(",", " ").strip()


def main() -> int:
    if len(sys.argv) != 3:
        print(__doc__.strip(), file=sys.stderr)
        return 2

    source = Path(sys.argv[1])
    output = Path(sys.argv[2])
    output.parent.mkdir(parents=True, exist_ok=True)

    with gzip.open(source, "rt", newline="", encoding="utf-8") as raw, output.open(
        "w", newline="", encoding="utf-8"
    ) as out:
        reader = csv.DictReader(raw)
        writer = csv.DictWriter(out, fieldnames=FIELDS)
        writer.writeheader()
        for row in reader:
            if row.get("proper") == "Sol":
                continue
            try:
                mag = float(row["mag"])
            except (TypeError, ValueError):
                continue
            if mag > 6.0:
                continue
            hip = row.get("hip", "").strip()
            if not hip:
                continue
            writer.writerow(
                {
                    "hip": hip,
                    "proper": clean_text(row.get("proper", "")),
                    "ra": row.get("ra", "").strip(),
                    "dec": row.get("dec", "").strip(),
                    "mag": f"{mag:.3f}",
                    "ci": row.get("ci", "").strip(),
                    "con": row.get("con", "").strip(),
                }
            )

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
