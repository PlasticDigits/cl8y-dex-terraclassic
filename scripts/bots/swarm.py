#!/usr/bin/env python3
"""
Continuous LocalTerra swap bot swarm.

- Inter-arrival times are exponential (Poisson process): sleep ~ Exp(rate=1/mean_seconds).
- Several bot *types* (different swap direction / amount policy / pair selection).
- Each type runs BOTS_REPLICAS_PER_TYPE concurrent workers (default 5) with slightly
  different mean intervals and amount multipliers.

Requires: docker, Python 3.10+ (stdlib only). LocalTerra container must be running.
Prerequisites: `make start` + `make deploy-local` (or equivalent) so factory + pairs exist.

Environment (optional overrides):
  FACTORY_ADDRESS / VITE_FACTORY_ADDRESS — factory contract (required)
  TERRA_LCD_URL — REST LCD (default http://127.0.0.1:1317)
  BOTS_MEAN_INTERVAL_SEC — base mean wait between swaps per worker (default 45)
  BOTS_REPLICAS_PER_TYPE — workers per type (default 5)
  BOTS_DIRECTED_SYMBOLS — comma symbols for "directed" bot pair, default OPAL,AMBER
  BOTS_DRY_RUN — set to 1 to log actions without broadcasting txs
  DEX_TERRA_RPC_PORT / DEX_TERRA_LCD_PORT — if using qa-shared-host remapped ports
"""

from __future__ import annotations

import argparse
import asyncio
import base64
import json
import os
import random
import subprocess
import sys
import urllib.error
import urllib.request
from dataclasses import dataclass
from typing import Any


def _env(name: str, default: str | None = None) -> str | None:
    v = os.environ.get(name)
    if v is not None and v.strip():
        return v.strip()
    return default


def _lcd_base() -> str:
    port = _env("DEX_TERRA_LCD_PORT", "1317")
    return (_env("TERRA_LCD_URL") or f"http://127.0.0.1:{port}").rstrip("/")


def _docker_localterra_id() -> str:
    """Return docker container id for service localterra (compose v2)."""
    repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
    r = subprocess.run(
        ["docker", "compose", "ps", "-q", "localterra"],
        cwd=repo_root,
        capture_output=True,
        text=True,
        check=False,
    )
    cid = (r.stdout or "").strip().splitlines()[0] if r.stdout else ""
    if not cid:
        print("ERROR: no localterra container id. Run `make start` from repo root.", file=sys.stderr)
        sys.exit(1)
    return cid


def _factory_addr() -> str:
    a = _env("FACTORY_ADDRESS") or _env("VITE_FACTORY_ADDRESS")
    if not a:
        env_local = os.path.abspath(
            os.path.join(os.path.dirname(__file__), "..", "..", "frontend-dapp", ".env.local")
        )
        if os.path.isfile(env_local):
            with open(env_local, encoding="utf-8") as f:
                for line in f:
                    line = line.strip()
                    if line.startswith("VITE_FACTORY_ADDRESS="):
                        a = line.split("=", 1)[1].strip().strip('"').strip("'")
                        break
    if not a:
        print("ERROR: set FACTORY_ADDRESS or VITE_FACTORY_ADDRESS (or frontend-dapp/.env.local).", file=sys.stderr)
        sys.exit(1)
    return a


def _b64_json(obj: dict[str, Any]) -> str:
    raw = json.dumps(obj, separators=(",", ":")).encode("utf-8")
    return base64.b64encode(raw).decode("ascii")


def _lcd_get_json(url: str) -> dict[str, Any]:
    req = urllib.request.Request(url, headers={"Accept": "application/json"})
    with urllib.request.urlopen(req, timeout=30) as resp:
        return json.loads(resp.read().decode("utf-8"))


def _lcd_smart(lcd: str, contract: str, msg: dict[str, Any]) -> dict[str, Any]:
    b64 = _b64_json(msg)
    url = f"{lcd}/cosmwasm/wasm/v1/contract/{contract}/smart/{b64}"
    return _lcd_get_json(url).get("data") or {}


def _iter_factory_pairs(lcd: str, factory: str) -> list[str]:
    """Paginate factory `pairs` query (max 30 per page). `start_after` is the pair's asset_infos tuple."""
    out: list[str] = []
    start_after: list[dict[str, Any]] | None = None
    while True:
        inner: dict[str, Any] = {"limit": 30}
        if start_after is not None:
            inner["start_after"] = start_after
        data = _lcd_smart(lcd, factory, {"pairs": inner})
        pairs = data.get("pairs") or []
        if not pairs:
            break
        for p in pairs:
            addr = p.get("contract_addr") or p.get("address")
            if isinstance(addr, str) and addr.startswith("terra1"):
                out.append(addr)
        if len(pairs) < 30:
            break
        last = pairs[-1]
        ai = last.get("asset_infos")
        if isinstance(ai, list) and len(ai) == 2:
            start_after = ai  # type: ignore[assignment]
        else:
            break
    return out


def _asset_contract(asset: dict[str, Any]) -> str | None:
    info = asset.get("info") or {}
    tok = info.get("token") or {}
    addr = tok.get("contract_addr")
    return addr if isinstance(addr, str) else None


def _asset_amount(asset: dict[str, Any]) -> int:
    raw = asset.get("amount", "0")
    try:
        return int(raw)
    except (TypeError, ValueError):
        return 0


def _cw20_symbol(lcd: str, cw20: str) -> str:
    try:
        data = _lcd_smart(lcd, cw20, {"token_info": {}})
        sym = data.get("symbol")
        return sym if isinstance(sym, str) else ""
    except (urllib.error.HTTPError, urllib.error.URLError, json.JSONDecodeError, KeyError):
        return ""


@dataclass
class PairMeta:
    pair_addr: str
    token0: str
    token1: str
    reserve0: int
    reserve1: int
    sym0: str
    sym1: str


def _load_pair_meta(lcd: str, pair_addr: str) -> PairMeta | None:
    try:
        pool = _lcd_smart(lcd, pair_addr, {"pool": {}})
    except (urllib.error.HTTPError, urllib.error.URLError, json.JSONDecodeError):
        return None
    assets = pool.get("assets")
    if not isinstance(assets, list) or len(assets) < 2:
        return None
    a0, a1 = assets[0], assets[1]
    t0 = _asset_contract(a0)
    t1 = _asset_contract(a1)
    if not t0 or not t1:
        return None
    r0, r1 = _asset_amount(a0), _asset_amount(a1)
    s0, s1 = _cw20_symbol(lcd, t0), _cw20_symbol(lcd, t1)
    return PairMeta(pair_addr, t0, t1, r0, r1, s0.upper(), s1.upper())


def _pick_directed(
    metas: list[PairMeta], want: tuple[str, str]
) -> PairMeta | None:
    a, b = want[0].upper(), want[1].upper()
    for m in metas:
        syms = {m.sym0, m.sym1}
        if a in syms and b in syms:
            return m
    return None


def _swap_hook_b64() -> str:
    msg = {"swap": {"belief_price": None, "max_spread": "0.50", "to": None, "deadline": None, "trader": None}}
    raw = json.dumps(msg, separators=(",", ":")).encode("utf-8")
    return base64.b64encode(raw).decode("ascii")


async def _terrad_tx(container: str, args: list[str]) -> tuple[int, str]:
    cmd = ["docker", "exec", container, "terrad", "tx", *args]
    proc = await asyncio.create_subprocess_exec(
        *cmd,
        stdout=asyncio.subprocess.PIPE,
        stderr=asyncio.subprocess.STDOUT,
    )
    out_b, _ = await proc.communicate()
    text = (out_b or b"").decode("utf-8", errors="replace")
    return proc.returncode or 0, text


async def swap_cw20_send(
    container: str,
    token: str,
    pair: str,
    amount: int,
    dry_run: bool,
) -> None:
    if amount < 1:
        amount = 1000
    hook = _swap_hook_b64()
    exec_msg = json.dumps(
        {"send": {"contract": pair, "amount": str(amount), "msg": hook}},
        separators=(",", ":"),
    )
    base = [
        "wasm",
        "execute",
        token,
        exec_msg,
        "--from",
        "test1",
        "--keyring-backend",
        "test",
        "--chain-id",
        "localterra",
        "--gas",
        "auto",
        "--gas-adjustment",
        "1.3",
        "--fees",
        "500000000uluna",
        "--node",
        "http://127.0.0.1:26657",
        "--broadcast-mode",
        "sync",
        "-y",
        "--output",
        "json",
    ]
    if dry_run:
        print(f"[dry-run] terrad tx wasm execute {token} send->{pair} amount={amount}")
        return
    code, out = await _terrad_tx(container, base)
    if code != 0:
        print(f"[warn] terrad exit {code}: {out[:500]}")


def _amount_from_reserves(reserve: int, mult: float, jitter: float) -> int:
    if reserve <= 0:
        return 1_000_000
    base = max(int(reserve * 0.002 * mult * jitter), 1_000)
    return min(base, int(reserve * 0.05))


BOT_TYPES = ("offer0", "offer1", "heavy", "light", "directed")


async def worker_loop(
    name: str,
    bot_type: str,
    replica_idx: int,
    metas: list[PairMeta],
    directed: PairMeta | None,
    container: str,
    mean_base: float,
    dry_run: bool,
    *,
    fixed_mean: bool = False,
) -> None:
    """One Poisson process: exponential inter-arrival with mean ~ mean_base (or scaled when not fixed_mean)."""
    if fixed_mean:
        # launch-swarm.sh sets BOTS_MEAN_INTERVAL_SEC per process as the actual Poisson mean.
        mean = mean_base
        env_amt = _env("BOTS_WORKER_AMOUNT_MULT")
        amt_mult = float(env_amt) if env_amt else (0.62 + 0.09 * replica_idx)
    else:
        mean = mean_base * (0.55 + 0.18 * replica_idx + random.random() * 0.08)
        env_amt = _env("BOTS_WORKER_AMOUNT_MULT")
        if env_amt:
            amt_mult = float(env_amt)
        else:
            amt_mult = 0.65 + 0.14 * replica_idx + random.random() * 0.06
    rng = random.Random(abs(hash(name)) % (2**31))

    while True:
        wait = rng.expovariate(1.0 / mean)
        await asyncio.sleep(min(max(wait, 0.5), 600.0))

        if bot_type == "directed" and directed is not None:
            m = directed
        else:
            m = rng.choice(metas)

        jitter = 0.85 + rng.random() * 0.3
        try:
            if bot_type == "offer0":
                amt = _amount_from_reserves(m.reserve0, amt_mult, jitter)
                await swap_cw20_send(container, m.token0, m.pair_addr, amt, dry_run)
            elif bot_type == "offer1":
                amt = _amount_from_reserves(m.reserve1, amt_mult, jitter)
                await swap_cw20_send(container, m.token1, m.pair_addr, amt, dry_run)
            elif bot_type == "heavy":
                side = rng.choice([0, 1])
                res = m.reserve0 if side == 0 else m.reserve1
                tok = m.token0 if side == 0 else m.token1
                amt = _amount_from_reserves(res, amt_mult * 2.2, jitter)
                await swap_cw20_send(container, tok, m.pair_addr, amt, dry_run)
            elif bot_type == "light":
                side = rng.choice([0, 1])
                res = m.reserve0 if side == 0 else m.reserve1
                tok = m.token0 if side == 0 else m.token1
                amt = max(int(_amount_from_reserves(res, amt_mult * 0.35, jitter)), 500)
                await swap_cw20_send(container, tok, m.pair_addr, amt, dry_run)
            else:
                # directed — prefer directed pair when configured
                side = rng.choice([0, 1])
                res = m.reserve0 if side == 0 else m.reserve1
                tok = m.token0 if side == 0 else m.token1
                amt = _amount_from_reserves(res, amt_mult, jitter)
                await swap_cw20_send(container, tok, m.pair_addr, amt, dry_run)
            print(f"[{name}] swap cycle (type={bot_type} pair={m.pair_addr[:18]}…)")
        except Exception as exc:  # noqa: BLE001 — long-running worker must not exit
            print(f"[{name}] error: {exc}", file=sys.stderr)


async def main_async_single(bot_type: str, replica_idx: int) -> None:
    """Run one worker process (used by launch-swarm.sh)."""
    if bot_type not in BOT_TYPES:
        print(f"ERROR: unknown bot type {bot_type!r}. Expected one of {BOT_TYPES}.", file=sys.stderr)
        sys.exit(2)
    lcd = _lcd_base()
    factory = _factory_addr()
    container = _docker_localterra_id()
    mean_base = float(_env("BOTS_MEAN_INTERVAL_SEC", "45") or "45")
    dry = (_env("BOTS_DRY_RUN", "0") or "0") == "1"

    sym_raw = _env("BOTS_DIRECTED_SYMBOLS", "OPAL,AMBER") or "OPAL,AMBER"
    parts = [p.strip().upper() for p in sym_raw.split(",") if p.strip()]
    directed_syms = (parts[0], parts[1]) if len(parts) >= 2 else ("OPAL", "AMBER")

    addrs = _iter_factory_pairs(lcd, factory)
    if not addrs:
        print("ERROR: factory returned no pairs.", file=sys.stderr)
        sys.exit(1)

    metas: list[PairMeta] = []
    for pa in addrs:
        meta = _load_pair_meta(lcd, pa)
        if meta:
            metas.append(meta)
    if len(metas) < 1:
        print("ERROR: could not load any CW20/CW20 pair metadata from LCD.", file=sys.stderr)
        sys.exit(1)

    directed = _pick_directed(metas, directed_syms)
    name = f"{bot_type}-{replica_idx}"
    print(f"[{name}] single-worker mode mean_env=BOTS_MEAN_INTERVAL_SEC={mean_base} dry_run={dry}", flush=True)
    await worker_loop(
        name, bot_type, replica_idx, metas, directed, container, mean_base, dry, fixed_mean=True
    )


async def main_async() -> None:
    lcd = _lcd_base()
    factory = _factory_addr()
    container = _docker_localterra_id()
    mean_base = float(_env("BOTS_MEAN_INTERVAL_SEC", "45") or "45")
    replicas = int(_env("BOTS_REPLICAS_PER_TYPE", "5") or "5")
    dry = (_env("BOTS_DRY_RUN", "0") or "0") == "1"

    sym_raw = _env("BOTS_DIRECTED_SYMBOLS", "OPAL,AMBER") or "OPAL,AMBER"
    parts = [p.strip().upper() for p in sym_raw.split(",") if p.strip()]
    directed_syms = (parts[0], parts[1]) if len(parts) >= 2 else ("OPAL", "AMBER")

    addrs = _iter_factory_pairs(lcd, factory)
    if not addrs:
        print("ERROR: factory returned no pairs.", file=sys.stderr)
        sys.exit(1)

    metas: list[PairMeta] = []
    for pa in addrs:
        meta = _load_pair_meta(lcd, pa)
        if meta:
            metas.append(meta)

    if len(metas) < 1:
        print("ERROR: could not load any CW20/CW20 pair metadata from LCD.", file=sys.stderr)
        sys.exit(1)

    directed = _pick_directed(metas, directed_syms)
    if directed is None:
        print(
            f"[warn] no pair matching symbols {directed_syms}; 'directed' bots use random pairs.",
            file=sys.stderr,
        )

    print(
        f"Swarm: {len(metas)} pairs, LCD={lcd}, mean_interval≈{mean_base}s, "
        f"{replicas} replicas × {len(BOT_TYPES)} types, dry_run={dry}"
    )
    if directed:
        print(f"Directed pair: {directed.sym0}/{directed.sym1} -> {directed.pair_addr}")

    tasks: list[asyncio.Task[None]] = []
    for btype in BOT_TYPES:
        for i in range(replicas):
            tag = f"{btype}-{i}"
            tasks.append(
                asyncio.create_task(
                    worker_loop(tag, btype, i, metas, directed, container, mean_base, dry, fixed_mean=False),
                    name=tag,
                )
            )

    await asyncio.gather(*tasks)


def main() -> None:
    parser = argparse.ArgumentParser(description="LocalTerra Poisson swap swarm")
    parser.add_argument(
        "--worker",
        nargs=2,
        metavar=("TYPE", "REPLICA"),
        help="Run a single bot worker in this process, e.g. --worker offer0 0",
    )
    args = parser.parse_args()
    try:
        if args.worker:
            btype, rid = args.worker[0], int(args.worker[1])
            asyncio.run(main_async_single(btype, rid))
        else:
            asyncio.run(main_async())
    except KeyboardInterrupt:
        print("Stopped.")


if __name__ == "__main__":
    main()
