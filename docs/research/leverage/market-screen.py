#!/usr/bin/env python3
"""Reproduce the descriptive market screen in leverage.md; not a risk calibrator.

Uses public Binance monthly spot archives, validates SHA-256 checksums, and
excludes every return window spanning a missing hourly bar. Standard library only.
"""

import argparse
import concurrent.futures
import csv
import datetime
import hashlib
import io
import json
from pathlib import Path
import tempfile
import urllib.request
import zipfile


SERIES = {
    "BTCUSDT": ("2021-01", "2025-12"),
    "ETHUSDT": ("2021-01", "2025-12"),
    "USTCBUSD": ("2022-06", "2023-05"),
    "USTCUSDT": ("2023-06", "2025-12"),
}


def months(start, end):
    year, month = map(int, start.split("-"))
    while f"{year:04d}-{month:02d}" <= end:
        yield f"{year:04d}-{month:02d}"
        month += 1
        if month == 13:
            year += 1
            month = 1


def download(task, cache):
    symbol, month = task
    name = f"{symbol}-1h-{month}.zip"
    path = cache / name
    url = f"https://data.binance.vision/data/spot/monthly/klines/{symbol}/1h/{name}"
    if path.exists():
        data = path.read_bytes()
    else:
        with urllib.request.urlopen(url, timeout=30) as response:
            data = response.read()
        path.write_bytes(data)
    with urllib.request.urlopen(url + ".CHECKSUM", timeout=30) as response:
        checksum = response.read().decode().split()[0]
    digest = hashlib.sha256(data).hexdigest()
    if checksum != digest:
        raise ValueError(f"Checksum mismatch: {url}")
    rows = []
    with zipfile.ZipFile(io.BytesIO(data)) as archive:
        with archive.open(archive.namelist()[0]) as raw:
            for row in csv.reader(io.TextIOWrapper(raw)):
                timestamp = int(row[0])
                # Binance spot archives changed ms -> us in January 2025.
                if timestamp > 10**14:
                    timestamp //= 1000
                rows.append((timestamp // 1000 + 3600, float(row[4])))
    return symbol, rows, {"url": url, "sha256": digest, "rows": len(rows)}


def iso(timestamp):
    return datetime.datetime.fromtimestamp(
        timestamp, datetime.timezone.utc
    ).isoformat()


def summarize(rows):
    rows = sorted(rows)
    if not rows or len(rows) != len({timestamp for timestamp, _ in rows}):
        raise ValueError("Missing series or duplicate timestamps")
    gaps = []
    segments = []
    segment = 0
    for index, (timestamp, price) in enumerate(rows):
        if price <= 0:
            raise ValueError("Nonpositive close")
        if index and timestamp - rows[index - 1][0] != 3600:
            difference = timestamp - rows[index - 1][0]
            if difference < 3600 or difference % 3600:
                raise ValueError("Invalid bar spacing")
            segment += 1
            gaps.append({
                "after": iso(rows[index - 1][0]),
                "before": iso(timestamp),
                "missing_hours": difference // 3600 - 1,
            })
        segments.append(segment)
    result = {
        "first_close": iso(rows[0][0]),
        "last_close": iso(rows[-1][0]),
        "rows": len(rows),
        "gaps": gaps,
        "horizons": {},
    }
    for hours in (1, 24, 168):
        returns = [
            (rows[index + hours][1] / rows[index][1] - 1, index)
            for index in range(len(rows) - hours)
            if segments[index] == segments[index + hours]
        ]
        ordered = sorted(returns)
        worst, index = ordered[0]
        result["horizons"][str(hours)] = {
            "windows": len(returns),
            "worst_return": worst,
            "worst_from": iso(rows[index][0]),
            "worst_to": iso(rows[index + hours][0]),
            "p01_return": ordered[int(0.01 * (len(ordered) - 1))][0],
            "p001_return": ordered[int(0.001 * (len(ordered) - 1))][0],
            "below_minus10_pct_count": sum(value < -0.1 for value, _ in returns),
            "below_minus1_pct_count": sum(value < -0.01 for value, _ in returns),
        }
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--cache-dir", type=Path,
        default=Path(tempfile.gettempdir()) / "leverage-market-data",
    )
    parser.add_argument(
        "--output", type=Path,
        default=Path(__file__).with_name("market-screen.json"),
    )
    args = parser.parse_args()
    args.cache_dir.mkdir(parents=True, exist_ok=True)
    tasks = [
        (symbol, month)
        for symbol, (start, end) in SERIES.items()
        for month in months(start, end)
    ]
    all_rows = {symbol: [] for symbol in SERIES}
    sources = []
    # A missing archive or invalid checksum aborts; no partial report is written.
    with concurrent.futures.ThreadPoolExecutor(max_workers=8) as pool:
        for symbol, rows, source in pool.map(
            lambda task: download(task, args.cache_dir), tasks
        ):
            all_rows[symbol].extend(rows)
            sources.append(source)
    output = {
        "retrieved_utc": datetime.datetime.now(datetime.timezone.utc).isoformat(),
        "method": (
            "Hourly close-to-close overlapping returns. Every intermediate bar "
            "must exist. No execution costs or wrapper/quote basis adjustments. "
            "Quantiles use lower order statistic floor(q*(n-1))."
        ),
        "series": SERIES,
        "results": {symbol: summarize(rows) for symbol, rows in all_rows.items()},
        "failed_archives": [],
        "sources": sources,
    }
    args.output.write_text(json.dumps(output, indent=2) + "\n")
    print(f"Verified {len(sources)} archives; wrote {args.output}")


if __name__ == "__main__":
    main()
