#!/usr/bin/env python3
"""Read-only completed-day drift receipt. Exit 1=drift, 2=unavailable evidence.

No credentials, database writes, historical repricing or upstream submissions.
JSON decimals remain exact strings in the receipt. Llama display rounding has
a $1 absolute tolerance per day; absent/null days are never coerced to zero.
"""
import argparse
from concurrent.futures import ThreadPoolExecutor
from datetime import date, datetime, timedelta, timezone
from decimal import Decimal, getcontext
import json
from pathlib import Path
import subprocess
import urllib.error
import urllib.request

INDEXER = "https://indexer.dex.cl8y.com"
LLAMA = "https://api.llama.fi"
START = date(2026, 8, 17)
getcontext().prec = 80
TOKENS = {
    "ust1": "terra1f0eqgy9w7e5e7up97vjudqwx38tesf8ylx75x2lv3nwm0clry0pqmgfy72",
    "ustr": "terra1vy3kc0swag2rhn7jz6n72jp0l2ns0p6r6ez5grxq5uhj2rvs97fqfsetxv",
    "cl8y": "terra16wtml2q66g82fdkx66tap0qjkahqwp4lwq3ngtygacg5q0kzycgqvhpax3",
}


def fetch(url):
    request = urllib.request.Request(url, headers={"User-Agent": "cl8y-defillama-1354-readonly"})
    try:
        with urllib.request.urlopen(request, timeout=20) as response:
            body = response.read(8_000_001)
            if len(body) > 8_000_000:
                return {"url": url, "status": "unavailable", "reason": "response byte budget"}
            return {"url": url, "status": response.status,
                    "data": json.loads(body, parse_float=Decimal)}
    except (urllib.error.URLError, ValueError, TimeoutError, OSError) as error:
        # Do not serialize request/error bodies or headers.
        return {"url": url, "status": getattr(error, "code", "unavailable"),
                "reason": type(error).__name__}


def timestamp(day):
    return int(datetime.combine(day, datetime.min.time(), timezone.utc).timestamp())


def compare(value, stored):
    if value is None:
        return "feed_null"
    if stored is None:
        return "llama_day_missing"
    if abs(Decimal(str(value)) - Decimal(str(stored))) > Decimal("1"):
        return "drift_over_rounding"
    return "within_rounding"


def has_completeness(feed):
    total = feed.get("trade_count")
    if type(total) is not int or total < 0:
        return False
    counts = [feed.get(k) for k in ("unpriced_trade_count", "priced_trade_count")]
    valid = [type(c) is int and 0 <= c <= total for c in counts]
    return (all(valid) and sum(counts) == total) or (sum(valid) == 1 and
            all(ok or c is None for ok, c in zip(valid, counts)))


def summarize(rows):
    return {
        "days": len(rows),
        "feed_priced_volume_sum": str(sum((Decimal(r["volume_usd"]) for r in rows
                                          if r["volume_usd"] is not None), Decimal(0))),
        "feed_priced_fee_sum": str(sum((Decimal(r["daily_fees_usd"]) for r in rows
                                       if r["daily_fees_usd"] is not None), Decimal(0))),
        "protocol_priced_volume_sum_broader_catalog": str(sum((Decimal(r["protocol_volume"]["volume_usd"])
            for r in rows if r["protocol_volume"] and r["protocol_volume"]["volume_usd"] is not None), Decimal(0))),
        "protocol_priced_fee_sum_broader_catalog": str(sum((Decimal(r["protocol_fees"]["fees_usd"])
            for r in rows if r["protocol_fees"] and r["protocol_fees"]["fees_usd"] is not None), Decimal(0))),
        "llama_volume_sum": str(sum((Decimal(str(r["llama_volume"])) for r in rows
                                     if r["llama_volume"] is not None), Decimal(0))),
        "llama_fee_sum": str(sum((Decimal(str(r["llama_fees"])) for r in rows
                                  if r["llama_fees"] is not None), Decimal(0))),
        "null_volume_dates": [r["date"] for r in rows if r["volume_usd"] is None],
        "missing_llama_volume_dates": [r["date"] for r in rows if r["llama_volume"] is None],
        "fee_drift_dates": [r["date"] for r in rows if r["fee_comparison"] != "within_rounding"],
    }


def collect(start, end):
    days = [start + timedelta(days=i) for i in range((end - start).days + 1)]
    urls = [f"{INDEXER}/api/v1/defillama/daily?timestamp={timestamp(d)}" for d in days]
    urls += [f"{LLAMA}/summary/dexs/cl8y-dex", f"{LLAMA}/summary/fees/cl8y-dex",
             f"{LLAMA}/protocol/cl8y-dex", f"{INDEXER}/api/v1/protocol/volume/daily?grain=daily&limit=90",
             f"{INDEXER}/api/v1/protocol/fees/daily?grain=daily&limit=90",
             f"{INDEXER}/api/v1/protocol/fees?window=24h", f"{INDEXER}/health",
             "https://coins.llama.fi/prices/current/" + ",".join("terra:" + t for t in TOKENS.values())]
    with ThreadPoolExecutor(max_workers=4) as executor:
        fetched = list(executor.map(fetch, urls))
    extras = fetched[len(days):]
    data = [r.get("data", {}) for r in extras]
    charts = [{int(ts): value for ts, value in d.get("totalDataChart", [])} for d in data[:2]]
    series = [{r["utc_day"]: r for r in d.get("series", [])} for d in data[3:5]]
    rows = []
    unavailable = [r for r in fetched if r["status"] != 200]
    for day, receipt in zip(days, fetched):
        if receipt["status"] != 200:
            continue
        feed = receipt["data"]
        if feed.get("date") != day.isoformat() or feed.get("timestamp") != timestamp(day):
            unavailable.append({"url": receipt["url"], "reason": "daily identity mismatch"})
            continue
        vol, fees = feed["volume_usd"], feed["daily_fees_usd"]
        lv, lf = charts[0].get(timestamp(day)), charts[1].get(timestamp(day))
        protocol_volume = series[0].get(day.isoformat())
        # Mirror upstream #9907's narrow legacy-null recovery for this receipt.
        # This is a candidate readback, not a same-snapshot DB scope certificate.
        adapter_volume, adapter_basis = vol, "daily_feed"
        if vol is None:
            adapter_basis = "unavailable"
            if (protocol_volume and type(feed["trade_count"]) is int and feed["trade_count"] > 0
                    and protocol_volume["trade_count"] == feed["trade_count"]
                    and data[3].get("grain") == "daily" and data[3].get("timezone") == "UTC"
                    and data[3].get("methodology") == "protocol_catalog"
                    and protocol_volume["volume_usd"] is not None
                    and Decimal(protocol_volume["volume_usd"]) > 0):
                adapter_volume = protocol_volume["volume_usd"]
                adapter_basis = "upstream_9907_matching_count_protocol_fallback"
        rows.append({"date": day.isoformat(), "timestamp": timestamp(day), "volume_usd": vol,
                     "trade_count": feed["trade_count"],
                     "unpriced_trade_count": feed.get("unpriced_trade_count"),
                     "priced_trade_count": feed.get("priced_trade_count"),
                     "completeness_metadata_present": has_completeness(feed),
                     "daily_fees_usd": fees, "fees": feed["fees"],
                     "protocol_volume": protocol_volume,
                     "protocol_fees": series[1].get(day.isoformat()),
                     "llama_volume": lv, "llama_fees": lf,
                     "adapter_volume_candidate": adapter_volume, "adapter_volume_basis": adapter_basis,
                     "volume_comparison": compare(adapter_volume, lv) if extras[0]["status"] == 200 else "unavailable",
                     "fee_comparison": compare(fees, lf) if extras[1]["status"] == 200 else "unavailable"})
    tokens_usd = data[2].get("tokensInUsd", [])
    prices = data[7].get("coins", {})
    sources = [r["source"] for r in data[5].get("by_source", [])]
    missing_prices = [name for name, addr in TOKENS.items() if "terra:" + addr not in prices]
    failures = []
    if any(r["volume_usd"] is None and r["adapter_volume_candidate"] is not None for r in rows):
        failures.append("live feed nulls partially priced days recoverable by the matching-count upstream fallback")
    if any(not r["completeness_metadata_present"] for r in rows):
        failures.append("daily feed lacks priced/unpriced volume completeness metadata")
    if any(r["trade_count"] > 0 and r["volume_usd"] is not None and Decimal(r["volume_usd"]) <= 0
           for r in rows):
        failures.append("live feed reports zero/negative USD for an active volume day")
    missing_rollups = [str(day) for day, receipt in zip(days, fetched) if receipt["status"] == 404]
    if missing_rollups:
        failures.append("daily rollup missing at/after adapter start: " + ", ".join(missing_rollups))
    # The latest two completed days may be awaiting upstream ingestion.
    settled = [r for r in rows if date.fromisoformat(r["date"]) <= end - timedelta(days=2)]
    if any(r["volume_comparison"] not in ("within_rounding", "unavailable") or
           r["fee_comparison"] not in ("within_rounding", "unavailable") for r in settled):
        failures.append("settled stored volume/fee chart has missing, null or divergent days")
    missing_sources = sorted({"pair_creation", "sku_unlock", "settings_fee"} - set(sources))
    if missing_sources and extras[5]["status"] == 200:
        failures.append("missing fee sources: " + ", ".join(missing_sources))
    if missing_prices and extras[7]["status"] == 200:
        failures.append("missing Llama CW20 prices: " + ", ".join(missing_prices))
    return {"observed_at": datetime.now(timezone.utc).isoformat(),
            "start": str(start), "end": str(end), "window": "completed UTC days, inclusive",
            "revision": subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip(),
            "health": data[6], "sources": sources, "daily": rows,
            "upstream_volume_fix": "https://github.com/DefiLlama/dimension-adapters/pull/9907",
            "summary": summarize(rows), "anchored_issue_window": summarize([r for r in rows
                if "2026-08-26" <= r["date"] <= "2026-09-24"]),
            "llama_latest_tokens_usd": tokens_usd[-1] if tokens_usd else None,
            "llama_current_cw20_prices": data[7], "missing_cw20_prices": missing_prices,
            "failures": failures, "unavailable": unavailable,
            "limits": ["protocol series includes gems; its sum is not listing acceptance",
                       "public feed omits unpriced counts; DB is needed for exact priced lower bound",
                       "TVL token snapshot and current prices are not a same-block reserve valuation",
                       "no historical backfill, price submission or upstream reload performed"],
            "result": "FAIL" if failures else ("INCOMPLETE" if unavailable else "PUBLIC CHECKS PASS")}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--start", type=date.fromisoformat, default=START)
    parser.add_argument("--end", type=date.fromisoformat,
                        default=datetime.now(timezone.utc).date() - timedelta(days=1))
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if args.start < START or args.end >= datetime.now(timezone.utc).date() or not 0 <= (args.end - args.start).days < 400:
        parser.error("use at most 400 completed UTC days at/after adapter start")
    receipt = collect(args.start, args.end)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(receipt, indent=2, default=str) + "\n")
    print(f"{receipt['result']}: {len(receipt['daily'])} days, {len(receipt['failures'])} findings, "
          f"{len(receipt['unavailable'])} unavailable requests; receipt: {args.output}")
    return 1 if receipt["failures"] else (2 if receipt["unavailable"] else 0)


if __name__ == "__main__":
    raise SystemExit(main())
