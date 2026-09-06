"""Generate placeholder UNVEIL icons (navy canvas). Not final branding."""

from __future__ import annotations

import struct
import zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
ICON_DIR = ROOT / "apps" / "desktop" / "src-tauri" / "icons"
RGB = (8, 13, 22)
ACCENT = (98, 229, 242)


def _chunk(tag: bytes, data: bytes) -> bytes:
    crc = zlib.crc32(tag + data) & 0xFFFFFFFF
    return struct.pack(">I", len(data)) + tag + data + struct.pack(">I", crc)


def png_bytes(width: int, height: int) -> bytes:
    rows = []
    cx, cy = width // 2, height // 2
    radius = max(2, min(width, height) // 6)
    for y in range(height):
        row = [0]
        for x in range(width):
            color = RGB
            if (x - cx) ** 2 + (y - cy) ** 2 <= radius**2:
                color = ACCENT
            row.extend(color)
        rows.append(bytes(row))
    raw = b"".join(rows)
    ihdr = struct.pack(">IIBBBBB", width, height, 8, 2, 0, 0, 0)
    return (
        b"\x89PNG\r\n\x1a\n"
        + _chunk(b"IHDR", ihdr)
        + _chunk(b"IDAT", zlib.compress(raw, 9))
        + _chunk(b"IEND", b"")
    )


def png_as_ico(png: bytes, width: int, height: int) -> bytes:
    # PNG-in-ICO (Vista+). Fine for a placeholder until branding exists.
    count = 1
    header = struct.pack("<HHH", 0, 1, count)
    entry = struct.pack(
        "<BBBBHHII",
        width if width < 256 else 0,
        height if height < 256 else 0,
        0,
        0,
        1,
        32,
        len(png),
        22,
    )
    return header + entry + png


def main() -> None:
    ICON_DIR.mkdir(parents=True, exist_ok=True)
    (ICON_DIR / "32x32.png").write_bytes(png_bytes(32, 32))
    (ICON_DIR / "128x128.png").write_bytes(png_bytes(128, 128))
    (ICON_DIR / "128x128@2x.png").write_bytes(png_bytes(256, 256))
    icon_png = png_bytes(256, 256)
    (ICON_DIR / "icon.ico").write_bytes(png_as_ico(icon_png, 256, 256))
    print(f"wrote icons in {ICON_DIR}")


if __name__ == "__main__":
    main()
