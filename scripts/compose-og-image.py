#!/usr/bin/env python3
"""Recompose the community medallion onto a 1200x630 Open Graph card (GitLab #578).

Does not stretch the 1254x1254 source. The sharp square is pillarboxed onto a
blurred, darkened cover-crop of the same hall so the portrait and CL8Y wordmark
stay intact while the extra width continues the chart.
"""
from __future__ import annotations

import argparse
import sys
from pathlib import Path

from PIL import Image, ImageEnhance, ImageFilter

OG_WIDTH = 1200
OG_HEIGHT = 630
MAX_BYTES = 5_000_000
TARGET_BYTES = 1_000_000


def _repo_root() -> Path:
    return Path(__file__).resolve().parents[1]


def compose(src: Image.Image) -> Image.Image:
    if src.width != src.height:
        raise SystemExit(f"source must be square, got {src.width}x{src.height}")

    rgb = src.convert("RGB")

    cover_scale = max(OG_WIDTH / rgb.width, OG_HEIGHT / rgb.height)
    cover = rgb.resize(
        (round(rgb.width * cover_scale), round(rgb.height * cover_scale)),
        Image.Resampling.LANCZOS,
    )
    cx = (cover.width - OG_WIDTH) // 2
    cy = (cover.height - OG_HEIGHT) // 2
    backdrop = cover.crop((cx, cy, cx + OG_WIDTH, cy + OG_HEIGHT))
    backdrop = backdrop.filter(ImageFilter.GaussianBlur(radius=16))
    backdrop = ImageEnhance.Brightness(backdrop).enhance(0.52)

    fg_scale = OG_HEIGHT / rgb.height
    foreground = rgb.resize(
        (round(rgb.width * fg_scale), OG_HEIGHT),
        Image.Resampling.LANCZOS,
    )
    # Slightly left of center so the hall/chart reads in the extra width.
    x = max(0, (OG_WIDTH - foreground.width) // 2 - 80)
    canvas = backdrop.copy()
    canvas.paste(foreground, (x, 0))
    return canvas


def save_png(image: Image.Image, dest: Path) -> None:
    dest.parent.mkdir(parents=True, exist_ok=True)
    image.save(dest, format="PNG", optimize=True)
    size = dest.stat().st_size
    if size > MAX_BYTES:
        raise SystemExit(f"{dest} is {size} bytes (hard max {MAX_BYTES})")
    if size > TARGET_BYTES:
        print(f"warning: {dest} is {size} bytes (target < {TARGET_BYTES})", file=sys.stderr)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--source",
        type=Path,
        default=_repo_root() / "frontend-dapp/brand/community-opengraph-concept.png",
    )
    parser.add_argument(
        "--dest",
        type=Path,
        default=_repo_root() / "frontend-dapp/public/og-image.png",
    )
    args = parser.parse_args()
    with Image.open(args.source) as src:
        composed = compose(src)
    save_png(composed, args.dest)
    print(f"wrote {args.dest} ({composed.width}x{composed.height}, {args.dest.stat().st_size} bytes)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
