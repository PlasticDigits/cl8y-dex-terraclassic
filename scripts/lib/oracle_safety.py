#!/usr/bin/env python3
"""Fail-closed #1391 contract for financial consumers; no network or signing."""
import argparse
from datetime import datetime, timezone
from decimal import Decimal, InvalidOperation
import json
import sys

SOURCES = {"kucoin", "mexc", "coingecko"}
QUOTE_ASSETS = {"ustc": "USTC", "lunc": "LUNC", "vfdusd": "FDUSD"}
VENUS_VTOKEN = "0xc4ef4229fec74ccfe17b2bdef7715fac740ba0ba"


def positive_decimal(value):
    try:
        result = Decimal(str(value))
    except (InvalidOperation, ValueError) as exc:
        raise ValueError("invalid decimal price") from exc
    if not result.is_finite() or not Decimal("1e-12") <= result <= Decimal("1e6"):
        raise ValueError("price must be finite, positive and within 1e-12..1e6")
    return result


def age_seconds(value, now=None):
    if not isinstance(value, str):
        raise ValueError("missing observation timestamp")
    try:
        observed = datetime.fromisoformat(value.replace("Z", "+00:00"))
    except ValueError as exc:
        raise ValueError("invalid observation timestamp") from exc
    if observed.tzinfo is None:
        raise ValueError("timestamp needs timezone")
    age = ((now or datetime.now(timezone.utc)) - observed).total_seconds()
    if age < 0:
        raise ValueError("future observation timestamp")
    return age


def validate_quote(raw, ticker, now=None, max_age_seconds=300):
    if not isinstance(raw, dict):
        raise ValueError("oracle response must be a JSON object")
    if ticker not in QUOTE_ASSETS or raw.get("ticker") != ticker or raw.get("quote_asset") != QUOTE_ASSETS[ticker]:
        raise ValueError("oracle ticker/quote identity mismatch")
    if raw.get("quality") != "fresh":
        raise ValueError("oracle quality must be fresh")
    if raw.get("source_policy") != "cex-quorum-v1":
        raise ValueError("unknown oracle source policy")
    quorum, count = raw.get("quorum"), raw.get("source_count")
    if type(quorum) is not int or type(count) is not int or quorum < 2 or count < quorum:
        raise ValueError("oracle source quorum not met")
    accepted = raw.get("accepted_sources")
    if not isinstance(accepted, list) or any(not isinstance(s, str) for s in accepted):
        raise ValueError("missing accepted source identities")
    if len(accepted) != count or len(set(accepted)) != count or not set(accepted) <= SOURCES:
        raise ValueError("invalid accepted source identities")
    if ticker == "vfdusd" and set(accepted) != {"mexc", "coingecko"}:
        raise ValueError("FDUSD requires its two listed sources: MEXC and CoinGecko")
    ttl = raw.get("max_age_seconds")
    if type(ttl) is not int or ttl <= 0 or max_age_seconds <= 0:
        raise ValueError("invalid oracle freshness policy")
    if age_seconds(raw.get("as_of"), now) > min(ttl, max_age_seconds, 300):
        raise ValueError("stale oracle observation")
    return positive_decimal(raw.get("price_usd"))


def validate_venus(raw, now=None):
    if not isinstance(raw, dict):
        raise ValueError("Venus response must be a JSON object")
    if raw.get("quality") != "fresh":
        raise ValueError("Venus quality must be fresh")
    if raw.get("source") != "venus_bsc" or str(raw.get("vtoken", "")).lower() != VENUS_VTOKEN:
        raise ValueError("Venus source or contract identity mismatch")
    if age_seconds(raw.get("fetched_at"), now) > 300:
        raise ValueError("stale Venus observation")
    return positive_decimal(raw.get("fdusd_per_vfdusd"))


def check_movement(current, planned):
    if abs(positive_decimal(current) / positive_decimal(planned) - 1) > Decimal("0.01"):
        raise ValueError("oracle moved more than 1%; recompute the transaction plan")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=("quote", "venus", "override", "movement"))
    parser.add_argument("args", nargs="*")
    args = parser.parse_args()
    try:
        if args.mode == "quote":
            result = validate_quote(json.load(sys.stdin), args.args[0], max_age_seconds=int(args.args[1]) if len(args.args) > 1 else 300)
        elif args.mode == "venus":
            result = validate_venus(json.load(sys.stdin))
        elif args.mode == "override":
            result = positive_decimal(args.args[0])
        else:
            check_movement(*args.args)
            return
        print(result)
    except (ValueError, KeyError, IndexError, TypeError) as exc:
        print(f"oracle safety: {exc}", file=sys.stderr)
        raise SystemExit(1) from exc


if __name__ == "__main__":
    main()
