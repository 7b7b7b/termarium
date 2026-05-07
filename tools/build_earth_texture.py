#!/usr/bin/env python3
"""Build Termarium's compact offline Earth surface texture.

Source data:
  NASA Blue Marble Next Generation, July 2004, topography and bathymetry
  https://science.nasa.gov/earth/earth-observatory/blue-marble-next-generation/

Usage:
  python3 tools/build_earth_texture.py /path/to/world.topo.bathy.200407.3x5400x2700.jpg data/earth_720x360.rgb
"""

from __future__ import annotations

import argparse
import subprocess
import tempfile
from pathlib import Path


def read_bmp_rgb(path: Path, width: int, height: int) -> bytes:
    raw = path.read_bytes()
    if raw[:2] != b"BM":
        raise ValueError(f"{path} is not a BMP file")

    offset = int.from_bytes(raw[10:14], "little")
    dib_size = int.from_bytes(raw[14:18], "little")
    if dib_size < 40:
        raise ValueError("unsupported BMP DIB header")

    bmp_width = int.from_bytes(raw[18:22], "little", signed=True)
    bmp_height = int.from_bytes(raw[22:26], "little", signed=True)
    planes = int.from_bytes(raw[26:28], "little")
    bits_per_pixel = int.from_bytes(raw[28:30], "little")
    compression = int.from_bytes(raw[30:34], "little")
    if planes != 1 or bits_per_pixel != 24 or compression != 0:
        raise ValueError("expected uncompressed 24-bit BMP from sips")
    if abs(bmp_width) != width or abs(bmp_height) != height:
        raise ValueError(f"expected {width}x{height}, got {bmp_width}x{bmp_height}")

    top_down = bmp_height < 0
    row_stride = ((width * 3 + 3) // 4) * 4
    rgb = bytearray(width * height * 3)
    for y in range(height):
        source_y = y if top_down else height - 1 - y
        row_start = offset + source_y * row_stride
        for x in range(width):
            source = row_start + x * 3
            target = (y * width + x) * 3
            blue, green, red = raw[source : source + 3]
            rgb[target : target + 3] = bytes((red, green, blue))
    return bytes(rgb)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--width", type=int, default=720)
    parser.add_argument("--height", type=int, default=360)
    args = parser.parse_args()

    args.output.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="termarium-earth-") as tmp:
        bmp = Path(tmp) / "earth.bmp"
        subprocess.run(
            [
                "sips",
                "-s",
                "format",
                "bmp",
                "-z",
                str(args.height),
                str(args.width),
                str(args.source),
                "--out",
                str(bmp),
            ],
            check=True,
            stdout=subprocess.DEVNULL,
        )
        args.output.write_bytes(read_bmp_rgb(bmp, args.width, args.height))

    print(f"earth texture: {args.width}x{args.height}")
    print(f"earth bytes: {args.output.stat().st_size}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
