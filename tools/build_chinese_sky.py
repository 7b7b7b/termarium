#!/usr/bin/env python3
"""Build compact Chinese sky-culture data for Termarium.

Usage:
  python3 tools/build_chinese_sky.py \
      /path/to/stellarium/chinese/index.json \
      /path/to/celestial_constellations_cn.csv \
      /path/to/celestial_starnames_cn.csv \
      data
"""

from __future__ import annotations

import csv
import json
import sys
from pathlib import Path


FIGURE_FIELDS = ["code", "zh", "pinyin", "en", "rank"]
LINE_FIELDS = ["code", "hips"]
STAR_NAME_FIELDS = ["hip", "zh", "pinyin", "en", "desig"]
REQUIRED_FIELDS = ["hip"]


def clean(value: str | None) -> str:
    return (value or "").replace(",", " ").strip()


def code_for(raw_id: str) -> str:
    prefix = "CON chinese "
    if not raw_id.startswith(prefix):
        raise ValueError(f"unexpected Chinese figure id: {raw_id}")
    return f"CN{int(raw_id.removeprefix(prefix)):03d}"


def numeric_id(raw_id: str) -> str:
    prefix = "CON chinese "
    if not raw_id.startswith(prefix):
        raise ValueError(f"unexpected Chinese figure id: {raw_id}")
    return str(int(raw_id.removeprefix(prefix)))


def load_celestial_figures(path: Path) -> dict[str, dict[str, str]]:
    with path.open(newline="", encoding="utf-8") as raw:
        return {row["id"]: row for row in csv.DictReader(raw)}


def write_csv(path: Path, fields: list[str], rows: list[dict[str, str]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", newline="", encoding="utf-8") as raw:
        writer = csv.DictWriter(raw, fieldnames=fields, lineterminator="\n")
        writer.writeheader()
        writer.writerows(rows)


def main() -> int:
    if len(sys.argv) != 5:
        print(__doc__.strip(), file=sys.stderr)
        return 2

    stellarium_index = Path(sys.argv[1])
    celestial_figures = Path(sys.argv[2])
    celestial_star_names = Path(sys.argv[3])
    output_dir = Path(sys.argv[4])

    source = json.loads(stellarium_index.read_text(encoding="utf-8"))
    celestial = load_celestial_figures(celestial_figures)

    figure_rows: list[dict[str, str]] = []
    line_rows: list[dict[str, str]] = []
    required_hips: set[int] = set()

    for figure in source.get("constellations", []):
        code = code_for(figure.get("id", ""))
        index_id = numeric_id(figure.get("id", ""))
        names = figure.get("common_name", {})
        celestial_row = celestial.get(index_id, {})
        figure_rows.append(
            {
                "code": code,
                "zh": clean(names.get("native") or celestial_row.get("name")),
                "pinyin": clean(celestial_row.get("pinyin") or names.get("pronounce")),
                "en": clean(names.get("english") or celestial_row.get("en")),
                "rank": clean(celestial_row.get("rank")),
            }
        )

        for raw_line in figure.get("lines", []):
            hips = [value for value in raw_line if isinstance(value, int)]
            if len(hips) < 2:
                continue
            required_hips.update(hips)
            line_rows.append(
                {
                    "code": code,
                    "hips": " ".join(str(hip) for hip in hips),
                }
            )

    star_name_rows: list[dict[str, str]] = []
    with celestial_star_names.open(newline="", encoding="utf-8") as raw:
        for row in csv.DictReader(raw):
            hip = row.get("id", "").strip()
            if not hip.isdigit():
                continue
            required_hips.add(int(hip))
            star_name_rows.append(
                {
                    "hip": hip,
                    "zh": clean(row.get("name")),
                    "pinyin": clean(row.get("pinyin")),
                    "en": clean(row.get("en")),
                    "desig": clean(row.get("desig")),
                }
            )

    write_csv(output_dir / "chinese_sky_figures.csv", FIGURE_FIELDS, figure_rows)
    write_csv(output_dir / "chinese_sky_lines.csv", LINE_FIELDS, line_rows)
    write_csv(output_dir / "chinese_star_names.csv", STAR_NAME_FIELDS, star_name_rows)
    write_csv(
        output_dir / "chinese_required_hips.csv",
        REQUIRED_FIELDS,
        [{"hip": str(hip)} for hip in sorted(required_hips)],
    )

    print(f"Chinese sky figures: {len(figure_rows)}")
    print(f"Chinese line paths: {len(line_rows)}")
    print(f"Chinese star names: {len(star_name_rows)}")
    print(f"Required HIP entries: {len(required_hips)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
