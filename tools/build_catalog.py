#!/usr/bin/env python3
"""Build Termarium's offline bright-star catalog from HYG v4.2.

Usage:
  python3 tools/build_catalog.py /path/to/hyg_v42.csv.gz data/hyg_v42_bright.csv
  python3 tools/build_catalog.py /path/to/hyg_v42.csv.gz data/hyg_v42_bright.csv \
      --include-hips data/chinese_required_hips.csv \
      --supplement data/hyg_chinese_supplement.csv
"""

from __future__ import annotations

import csv
import gzip
import sys
from pathlib import Path
from typing import TextIO


FIELDS = ["hip", "proper", "ra", "dec", "mag", "ci", "con"]


def clean_text(value: str) -> str:
    return value.replace(",", " ").strip()


def open_source(path: Path) -> TextIO:
    if path.suffix == ".gz":
        return gzip.open(path, "rt", newline="", encoding="utf-8")
    return path.open("r", newline="", encoding="utf-8")


def read_include_hips(path: Path | None) -> set[int]:
    if path is None:
        return set()
    with path.open(newline="", encoding="utf-8") as raw:
        reader = csv.DictReader(raw)
        return {
            int(row["hip"])
            for row in reader
            if row.get("hip", "").strip().isdigit()
        }


def parse_args(argv: list[str]) -> tuple[Path, Path, Path | None, Path | None]:
    if len(argv) not in {3, 7}:
        print(__doc__.strip(), file=sys.stderr)
        raise SystemExit(2)

    source = Path(argv[1])
    output = Path(argv[2])
    include_hips = None
    supplement = None

    if len(argv) == 7:
        if argv[3] != "--include-hips" or argv[5] != "--supplement":
            print(__doc__.strip(), file=sys.stderr)
            raise SystemExit(2)
        include_hips = Path(argv[4])
        supplement = Path(argv[6])

    return source, output, include_hips, supplement


def row_from_hyg(row: dict[str, str], mag: float) -> dict[str, str]:
    return {
        "hip": row.get("hip", "").strip(),
        "proper": clean_text(row.get("proper", "")),
        "ra": row.get("ra", "").strip(),
        "dec": row.get("dec", "").strip(),
        "mag": f"{mag:.3f}",
        "ci": row.get("ci", "").strip(),
        "con": row.get("con", "").strip(),
    }


def load_supplements(path: Path | None) -> list[dict[str, str]]:
    if path is None:
        return []
    with path.open(newline="", encoding="utf-8") as raw:
        return list(csv.DictReader(raw))


def main() -> int:
    source, output, include_hips_path, supplement_path = parse_args(sys.argv)
    include_hips = read_include_hips(include_hips_path)
    output.parent.mkdir(parents=True, exist_ok=True)
    rows: list[dict[str, str]] = []
    emitted_hips: set[int] = set()

    with open_source(source) as raw:
        reader = csv.DictReader(raw)
        for row in reader:
            if row.get("proper") == "Sol":
                continue
            hip = row.get("hip", "").strip()
            if not hip:
                continue
            try:
                mag = float(row["mag"])
            except (TypeError, ValueError):
                continue
            hip_value = int(hip)
            if mag > 6.0 and hip_value not in include_hips:
                continue
            rows.append(row_from_hyg(row, mag))
            emitted_hips.add(hip_value)

    for row in load_supplements(supplement_path):
        hip = row.get("hip", "").strip()
        if not hip or int(hip) in emitted_hips:
            continue
        rows.append({field: row.get(field, "").strip() for field in FIELDS})
        emitted_hips.add(int(hip))

    with output.open("w", newline="", encoding="utf-8") as out:
        writer = csv.DictWriter(out, fieldnames=FIELDS, lineterminator="\n")
        writer.writeheader()
        writer.writerows(rows)

    missing = sorted(include_hips - emitted_hips)
    if missing:
        print(f"warning: missing requested HIP entries: {missing}", file=sys.stderr)

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
