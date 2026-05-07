#!/usr/bin/env python3
"""Build Termarium's offline constellation-line catalog.

Usage:
  python3 tools/build_constellation_lines.py \
      /path/to/hyg_v42.csv.gz \
      /path/to/ConstellationLines.csv \
      data/constellation_lines_hip.csv

The source constellation lines use Bright Star Catalogue / Harvard Revised
numbers. Termarium renders stars by HIP number, so this script maps HR to HIP
through the full HYG catalog and keeps only stars that are present in the
bundled bright-star subset.
"""

from __future__ import annotations

import csv
import gzip
import sys
from collections import Counter
from pathlib import Path


FIELDS = ["code", "hips"]
MAX_MAGNITUDE = 6.0


def load_hyg_maps(source: Path) -> tuple[dict[int, int], set[int]]:
    hr_to_hip: dict[int, int] = {}
    bright_hips: set[int] = set()

    with gzip.open(source, "rt", newline="", encoding="utf-8") as raw:
        reader = csv.DictReader(raw)
        for row in reader:
            hip_raw = row.get("hip", "").strip()
            hr_raw = row.get("hr", "").strip()
            if hip_raw and hr_raw:
                hr_to_hip[int(hr_raw)] = int(hip_raw)

            if row.get("proper") == "Sol" or not hip_raw:
                continue
            try:
                magnitude = float(row.get("mag", ""))
            except ValueError:
                continue
            if magnitude <= MAX_MAGNITUDE:
                bright_hips.add(int(hip_raw))

    return hr_to_hip, bright_hips


def star_columns(row: dict[str, str]) -> list[str]:
    return sorted(key for key in row if key and key.startswith("s"))


def flush_path(rows: list[dict[str, str]], code: str, hips: list[int]) -> None:
    if len(hips) >= 2:
        rows.append({"code": code, "hips": " ".join(str(hip) for hip in hips)})


def convert_lines(
    source: Path,
    hr_to_hip: dict[int, int],
    bright_hips: set[int],
) -> tuple[list[dict[str, str]], dict[str, int]]:
    rows: list[dict[str, str]] = []
    previous_code = ""
    source_rows = 0
    source_codes: set[str] = set()
    missing_hr: Counter[int] = Counter()
    filtered_dim = 0
    split_paths = 0

    with source.open(newline="", encoding="utf-8") as raw:
        reader = csv.DictReader(raw, skipinitialspace=True)
        for row in reader:
            source_rows += 1
            code = (row.get("abr") or "").strip() or previous_code
            if not code:
                raise ValueError("first constellation-line row has no abbreviation")
            previous_code = code
            source_codes.add(code)

            path: list[int] = []
            for column in star_columns(row):
                value = (row.get(column) or "").strip()
                if not value:
                    continue

                hr = int(value)
                hip = hr_to_hip.get(hr)
                if hip is None:
                    missing_hr[hr] += 1
                    flush_path(rows, code, path)
                    if path:
                        split_paths += 1
                    path = []
                    continue

                if hip not in bright_hips:
                    filtered_dim += 1
                    flush_path(rows, code, path)
                    if path:
                        split_paths += 1
                    path = []
                    continue

                path.append(hip)

            flush_path(rows, code, path)

    output_codes = {row["code"] for row in rows}
    stats = {
        "source_rows": source_rows,
        "source_codes": len(source_codes),
        "output_rows": len(rows),
        "output_codes": len(output_codes),
        "segments": sum(max(0, len(row["hips"].split()) - 1) for row in rows),
        "missing_hr_values": len(missing_hr),
        "missing_hr_references": sum(missing_hr.values()),
        "filtered_dim_references": filtered_dim,
        "split_paths": split_paths,
    }
    if source_codes - output_codes:
        raise ValueError(f"constellations lost during conversion: {sorted(source_codes - output_codes)}")

    return rows, stats


def main() -> int:
    if len(sys.argv) != 4:
        print(__doc__.strip(), file=sys.stderr)
        return 2

    hyg_source = Path(sys.argv[1])
    line_source = Path(sys.argv[2])
    output = Path(sys.argv[3])
    output.parent.mkdir(parents=True, exist_ok=True)

    hr_to_hip, bright_hips = load_hyg_maps(hyg_source)
    rows, stats = convert_lines(line_source, hr_to_hip, bright_hips)

    with output.open("w", newline="", encoding="utf-8") as raw:
        writer = csv.DictWriter(raw, fieldnames=FIELDS)
        writer.writeheader()
        writer.writerows(rows)

    print(f"source rows: {stats['source_rows']}")
    print(f"source constellations: {stats['source_codes']}")
    print(f"output polylines: {stats['output_rows']}")
    print(f"output constellations: {stats['output_codes']}")
    print(f"output segments: {stats['segments']}")
    print(f"missing HR references: {stats['missing_hr_references']}")
    print(f"filtered dim-star references: {stats['filtered_dim_references']}")
    print(f"split paths: {stats['split_paths']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
