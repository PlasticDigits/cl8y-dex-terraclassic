#!/usr/bin/env python3
"""
Continuous LocalTerra swap bot swarm (+ optional resting limit orders).

- Inter-arrival times are exponential (Poisson process): sleep ~ Exp(rate=1/mean_seconds).
- Swap bot *types* (offer0/offer1/heavy/light/directed) only hit **AMM swaps** via CW20 `send`
  (Recent trades move; **order book** stays empty).
- `launch-swarm.sh` also starts **five** `--worker limit N` processes that place **resting**
  `place_limit_order` bids/asks so `/trade` order book panels are populated for QA.
- **Three** `--worker lp N` processes periodically `provide_liquidity` so swap-only bots do not
  drain LocalTerra test pools (GitLab #293). `launch-swarm.sh` runs `bootstrap-swarm-liquidity.sh`
  once before workers unless `BOTS_SKIP_BOOTSTRAP=1`.

Requires: docker, Python 3.10+ (stdlib `decimal` + asyncio). LocalTerra container must be running.
Prerequisites: `make start` + `make deploy-local` (or equivalent) so factory + pairs exist.

Environment (optional overrides):
  FACTORY_ADDRESS / VITE_FACTORY_ADDRESS — factory contract (required)
  TERRA_LCD_URL — REST LCD (default http://127.0.0.1:1317)
  BOTS_MEAN_INTERVAL_SEC — base mean wait between swaps per worker (default 45)
  BOTS_LIMIT_MEAN_INTERVAL_SEC — mean wait for **limit** workers (default 120)
  BOTS_LP_MEAN_INTERVAL_SEC — mean wait for **lp** workers (default 90)
  BOTS_MIN_RESERVE_PER_SIDE — skip swaps on pairs thinner than this (default 10_000_000)
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
from decimal import Decimal, getcontext
from typing import Any

from swarm_liquidity import (
    MIN_PROVIDE_LIQUIDITY_LEG,
    MIN_RESERVE_PER_SIDE_FOR_SWAP,
    bootstrap_top_up_amounts,
    min_leg_provide_amounts,
    pick_scaled_provide_amounts,
    pool_reserves_ok,
)


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


TEST1_ADDRESS = "terra1x46rqay4d3cssq8gxxvqz8xt6nwlz4td20k38v"
# 100k LUNC in uluna (6 decimals) — warn when QA swarm may soon fail signing (GitLab #372).
LOW_GAS_WARN_ULUNA = 100_000 * 1_000_000


def _query_test1_uluna(lcd: str) -> int | None:
    """Return test1 uluna balance or None if LCD query fails."""
    try:
        url = f"{lcd}/cosmos/bank/v1beta1/balances/{TEST1_ADDRESS}"
        data = _lcd_get_json(url)
        for bal in data.get("balances") or []:
            if bal.get("denom") == "uluna":
                return int(bal.get("amount") or "0")
        return 0
    except (urllib.error.HTTPError, urllib.error.URLError, json.JSONDecodeError, ValueError):
        return None


def preflight_gas_balance() -> int:
    """Query test1 uluna; warn below LOW_GAS_WARN_ULUNA. Returns balance or -1 on LCD failure."""
    lcd = _lcd_base()
    amount = _query_test1_uluna(lcd)
    if amount is None:
        print(f"WARN: could not query test1 uluna balance on {lcd}", file=sys.stderr)
        return -1
    lunc = amount / 1_000_000
    print(f"[preflight] test1 uluna={amount} (~{lunc:,.0f} LUNC) LCD={lcd}", flush=True)
    if amount < LOW_GAS_WARN_ULUNA:
        print(
            f"WARN: test1 uluna below {LOW_GAS_WARN_ULUNA} (~100k LUNC) — "
            "swarm / simulated wallet may fail signing; run `make reset` (GitLab #372).",
            file=sys.stderr,
        )
    return amount


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


async def _terrad_query(container: str, args: list[str]) -> tuple[int, str]:
    cmd = ["docker", "exec", container, "terrad", "query", *args]
    proc = await asyncio.create_subprocess_exec(
        *cmd,
        stdout=asyncio.subprocess.PIPE,
        stderr=asyncio.subprocess.STDOUT,
    )
    out_b, _ = await proc.communicate()
    text = (out_b or b"").decode("utf-8", errors="replace")
    return proc.returncode or 0, text


def _min_reserve_per_side() -> int:
    raw = _env("BOTS_MIN_RESERVE_PER_SIDE")
    if raw:
        try:
            return max(int(raw), 1)
        except ValueError:
            pass
    return MIN_RESERVE_PER_SIDE_FOR_SWAP


def _txhash_from_terrad_json(text: str) -> str | None:
    # terrad may prefix stderr ("gas estimate: …") on the same stream as JSON stdout.
    data = _json_from_terrad_output(text)
    if data is None:
        return None
    txhash = data.get("txhash")
    if isinstance(txhash, str) and txhash:
        return txhash
    tx_resp = data.get("tx_response")
    if isinstance(tx_resp, dict):
        nested = tx_resp.get("txhash")
        if isinstance(nested, str) and nested:
            return nested
    return None


def _tx_query_succeeded(data: dict) -> bool:
    codes: list[int] = []
    if "code" in data:
        codes.append(int(data["code"]))
    tx_resp = data.get("tx_response")
    if isinstance(tx_resp, dict) and "code" in tx_resp:
        codes.append(int(tx_resp["code"]))
    return not codes or all(c == 0 for c in codes)


def _json_from_terrad_output(text: str) -> dict | None:
    stripped = text.strip()
    json_start = stripped.find("{")
    if json_start > 0:
        stripped = stripped[json_start:]
    try:
        data = json.loads(stripped)
    except json.JSONDecodeError:
        return None
    return data if isinstance(data, dict) else None


async def _poll_tx_inclusion(container: str, txhash: str, *, timeout_sec: float = 90.0) -> bool:
    deadline = asyncio.get_running_loop().time() + timeout_sec
    while asyncio.get_running_loop().time() < deadline:
        code, out = await _terrad_query(
            container,
            ["tx", txhash, "--node", "http://127.0.0.1:26657", "--output", "json"],
        )
        if code == 0:
            data = _json_from_terrad_output(out)
            if data is None:
                await asyncio.sleep(1.0)
                continue
            if not _tx_query_succeeded(data):
                tx_resp = data.get("tx_response")
                tx_code = tx_resp.get("code", data.get("code")) if isinstance(tx_resp, dict) else data.get("code")
                print(f"[warn] tx {txhash[:16]}… failed code={tx_code}: {out[:300]}")
                return False
            return True
        await asyncio.sleep(1.0)
    print(f"[warn] tx {txhash[:16]}… inclusion timeout after {timeout_sec:.0f}s")
    return False


async def _wasm_execute(
    container: str, contract: str, exec_msg: str, dry_run: bool, label: str, *, wait: bool = False
) -> bool:
    base = [
        "wasm",
        "execute",
        contract,
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
        print(f"[dry-run] terrad tx wasm execute {contract} {label}")
        return True
    code, out = await _terrad_tx(container, base)
    if code != 0:
        print(f"[warn] terrad exit {code}: {out[:500]}")
        return False
    if not wait:
        return True
    txhash = _txhash_from_terrad_json(out)
    if not txhash:
        print(f"[warn] no txhash in terrad output for {label}")
        return False
    return await _poll_tx_inclusion(container, txhash)


async def provide_liquidity_pair(
    container: str,
    token0: str,
    token1: str,
    pair: str,
    amount0: int,
    amount1: int,
    dry_run: bool,
) -> bool:
    if amount0 < 1 or amount1 < 1:
        return False
    allow0 = json.dumps(
        {"increase_allowance": {"spender": pair, "amount": str(amount0), "expires": {"never": {}}}},
        separators=(",", ":"),
    )
    allow1 = json.dumps(
        {"increase_allowance": {"spender": pair, "amount": str(amount1), "expires": {"never": {}}}},
        separators=(",", ":"),
    )
    provide = json.dumps(
        {
            "provide_liquidity": {
                "assets": [
                    {"info": {"token": {"contract_addr": token0}}, "amount": str(amount0)},
                    {"info": {"token": {"contract_addr": token1}}, "amount": str(amount1)},
                ],
                "slippage_tolerance": None,
                "receiver": None,
                "deadline": None,
            }
        },
        separators=(",", ":"),
    )
    if not await _wasm_execute(
        container, token0, allow0, dry_run, f"allow->{pair[:12]}… amt0={amount0}", wait=True
    ):
        return False
    if not await _wasm_execute(
        container, token1, allow1, dry_run, f"allow->{pair[:12]}… amt1={amount1}", wait=True
    ):
        return False
    return await _wasm_execute(
        container, pair, provide, dry_run, f"provide_liquidity amt0={amount0} amt1={amount1}", wait=True
    )


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
    await _wasm_execute(container, token, exec_msg, dry_run, f"send->{pair[:12]}… amount={amount}")


def _amount_from_reserves(reserve: int, mult: float, jitter: float) -> int:
    if reserve <= 0:
        return 1_000_000
    base = max(int(reserve * 0.002 * mult * jitter), 1_000)
    return min(base, int(reserve * 0.05))


def _collect_pair_metas(lcd: str, factory: str) -> list[PairMeta]:
    addrs = _iter_factory_pairs(lcd, factory)
    metas: list[PairMeta] = []
    for pa in addrs:
        meta = _load_pair_meta(lcd, pa)
        if meta:
            metas.append(meta)
    return metas


def _decimal_price_str(price: Decimal) -> str:
    """CosmWasm `Decimal` string: positive, no scientific notation."""
    getcontext().prec = 48
    if price <= 0:
        return "0.000001"
    q = price.quantize(Decimal("1E-18"))
    s = format(q, "f").rstrip("0").rstrip(".")
    return s if s else "0.000001"


async def place_limit_cw20_send(
    container: str,
    token: str,
    pair: str,
    amount: int,
    side: str,
    price_str: str,
    dry_run: bool,
) -> None:
    if amount < 1:
        return
    inner: dict[str, Any] = {
        "place_limit_order_batch": {
            "side": side,
            "orders": [
                {
                    "price": price_str,
                    "amount": str(amount),
                    "max_adjust_steps": 64,
                }
            ],
        }
    }
    hook = _b64_json(inner)
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
        print(f"[dry-run] terrad limit {side} token={token[:18]}… pair={pair[:18]}… amt={amount} price={price_str}")
        return
    code, out = await _terrad_tx(container, base)
    if code != 0:
        print(f"[warn] terrad limit exit {code}: {out[:500]}")


async def limit_order_worker_loop(
    name: str,
    replica_idx: int,
    metas: list[PairMeta],
    lcd: str,
    container: str,
    mean_base: float,
    dry: bool,
) -> None:
    """Place resting bids (token1) and asks (token0) off the pool mid price (token1 per token0)."""
    rng = random.Random(abs(hash(name)) % (2**31))
    env_amt = _env("BOTS_WORKER_AMOUNT_MULT")
    amt_mult = float(env_amt) if env_amt else (0.62 + 0.09 * replica_idx)
    min_escrow = int(_env("BOTS_LIMIT_MIN_ESCROW", "50000000") or "50000000")

    while True:
        wait = rng.expovariate(1.0 / mean_base)
        await asyncio.sleep(min(max(wait, 1.0), 600.0))

        m = rng.choice(metas)
        fresh = _load_pair_meta(lcd, m.pair_addr)
        if not fresh:
            continue
        m = fresh
        if m.reserve0 < 1 or m.reserve1 < 1:
            continue

        r0 = Decimal(m.reserve0)
        r1 = Decimal(m.reserve1)
        mid = r1 / r0
        if mid <= 0:
            continue

        side = rng.choice(["bid", "ask"])
        if side == "bid":
            factor = Decimal("0.82") + Decimal(str(rng.random() * 0.13))
            price = mid * factor
            escrow_tok = m.token1
            res = m.reserve1
        else:
            factor = Decimal("1.05") + Decimal(str(rng.random() * 0.18))
            price = mid * factor
            escrow_tok = m.token0
            res = m.reserve0

        price_str = _decimal_price_str(price)
        jitter = 0.85 + rng.random() * 0.3
        amt = _amount_from_reserves(res, amt_mult, jitter)
        amt = max(amt, min_escrow)

        try:
            await place_limit_cw20_send(container, escrow_tok, m.pair_addr, amt, side, price_str, dry)
            print(
                f"[{name}] limit {side} pair={m.sym0}/{m.sym1} price={price_str} amt={amt}",
                flush=True,
            )
        except Exception as exc:  # noqa: BLE001
            print(f"[{name}] error: {exc}", file=sys.stderr)


SWAP_BOT_TYPES = ("offer0", "offer1", "heavy", "light", "directed")
WORKER_TYPES = SWAP_BOT_TYPES + ("limit", "lp")


async def lp_worker_loop(
    name: str,
    replica_idx: int,
    metas: list[PairMeta],
    lcd: str,
    container: str,
    mean_base: float,
    dry: bool,
) -> None:
    """Periodically add proportional liquidity so swap bots do not leave thin pools (GitLab #293)."""
    rng = random.Random(abs(hash(name)) % (2**31))
    env_amt = _env("BOTS_LP_FRACTION_PPM")
    fraction_ppm = int(env_amt) if env_amt else 3_000

    while True:
        wait = rng.expovariate(1.0 / mean_base)
        await asyncio.sleep(min(max(wait, 1.0), 600.0))

        m = rng.choice(metas)
        fresh = _load_pair_meta(lcd, m.pair_addr)
        if not fresh:
            continue
        m = fresh
        floor = _min_reserve_per_side()
        scaled = pick_scaled_provide_amounts(
            m.reserve0, m.reserve1, fraction_ppm, min_reserve_per_side=floor
        )
        if scaled:
            amount0, amount1 = scaled
        else:
            min_leg = min_leg_provide_amounts(
                m.reserve0, m.reserve1, min_reserve_per_side=floor
            )
            if min_leg:
                amount0, amount1 = min_leg
            else:
                topped = bootstrap_top_up_amounts(
                    m.reserve0,
                    m.reserve1,
                    floor_per_side=floor,
                    target_per_side=floor + MIN_PROVIDE_LIQUIDITY_LEG,
                )
                if not topped:
                    continue
                amount0, amount1 = topped
        try:
            if await provide_liquidity_pair(
                container, m.token0, m.token1, m.pair_addr, amount0, amount1, dry
            ):
                print(
                    f"[{name}] provide_liquidity pair={m.sym0}/{m.sym1} "
                    f"amt0={amount0} amt1={amount1}",
                    flush=True,
                )
        except Exception as exc:  # noqa: BLE001
            print(f"[{name}] error: {exc}", file=sys.stderr)


async def worker_loop(
    name: str,
    bot_type: str,
    replica_idx: int,
    metas: list[PairMeta],
    directed: PairMeta | None,
    lcd: str,
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

        fresh = _load_pair_meta(lcd, m.pair_addr)
        if not fresh:
            continue
        m = fresh
        floor = _min_reserve_per_side()
        if m.reserve0 < floor or m.reserve1 < floor:
            continue

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


async def main_async_lp_only(replica_idx: int) -> None:
    """Single-process LP bot (`--worker lp N`)."""
    lcd = _lcd_base()
    factory = _factory_addr()
    container = _docker_localterra_id()
    mean_base = float(_env("BOTS_LP_MEAN_INTERVAL_SEC", "90") or "90")
    dry = (_env("BOTS_DRY_RUN", "0") or "0") == "1"
    metas = _collect_pair_metas(lcd, factory)
    if len(metas) < 1:
        print("ERROR: could not load any CW20/CW20 pair metadata from LCD.", file=sys.stderr)
        sys.exit(1)
    name = f"lp-{replica_idx}"
    print(f"[{name}] provide_liquidity worker mean={mean_base}s dry_run={dry}", flush=True)
    await lp_worker_loop(name, replica_idx, metas, lcd, container, mean_base, dry)


async def main_async_limit_only(replica_idx: int) -> None:
    """Single-process limit bot (`--worker limit N`)."""
    lcd = _lcd_base()
    factory = _factory_addr()
    container = _docker_localterra_id()
    mean_base = float(_env("BOTS_LIMIT_MEAN_INTERVAL_SEC", "120") or "120")
    dry = (_env("BOTS_DRY_RUN", "0") or "0") == "1"
    metas = _collect_pair_metas(lcd, factory)
    if len(metas) < 1:
        print("ERROR: could not load any CW20/CW20 pair metadata from LCD.", file=sys.stderr)
        sys.exit(1)
    name = f"limit-{replica_idx}"
    print(f"[{name}] limit-order worker mean={mean_base}s dry_run={dry}", flush=True)
    await limit_order_worker_loop(name, replica_idx, metas, lcd, container, mean_base, dry)


async def main_async_single(bot_type: str, replica_idx: int) -> None:
    """Run one worker process (used by launch-swarm.sh)."""
    if bot_type == "limit":
        await main_async_limit_only(replica_idx)
        return
    if bot_type == "lp":
        await main_async_lp_only(replica_idx)
        return
    if bot_type not in SWAP_BOT_TYPES:
        print(
            f"ERROR: unknown bot type {bot_type!r}. Expected one of {WORKER_TYPES}.",
            file=sys.stderr,
        )
        sys.exit(2)
    lcd = _lcd_base()
    factory = _factory_addr()
    container = _docker_localterra_id()
    mean_base = float(_env("BOTS_MEAN_INTERVAL_SEC", "45") or "45")
    dry = (_env("BOTS_DRY_RUN", "0") or "0") == "1"

    sym_raw = _env("BOTS_DIRECTED_SYMBOLS", "OPAL,AMBER") or "OPAL,AMBER"
    parts = [p.strip().upper() for p in sym_raw.split(",") if p.strip()]
    directed_syms = (parts[0], parts[1]) if len(parts) >= 2 else ("OPAL", "AMBER")

    metas = _collect_pair_metas(lcd, factory)
    if len(metas) < 1:
        print("ERROR: could not load any CW20/CW20 pair metadata from LCD.", file=sys.stderr)
        sys.exit(1)

    directed = _pick_directed(metas, directed_syms)
    name = f"{bot_type}-{replica_idx}"
    print(f"[{name}] single-worker mode mean_env=BOTS_MEAN_INTERVAL_SEC={mean_base} dry_run={dry}", flush=True)
    await worker_loop(
        name, bot_type, replica_idx, metas, directed, lcd, container, mean_base, dry, fixed_mean=True
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

    metas = _collect_pair_metas(lcd, factory)

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
        f"{replicas} replicas × {len(SWAP_BOT_TYPES)} swap types, dry_run={dry}"
    )
    if directed:
        print(f"Directed pair: {directed.sym0}/{directed.sym1} -> {directed.pair_addr}")

    tasks: list[asyncio.Task[None]] = []
    for btype in SWAP_BOT_TYPES:
        for i in range(replicas):
            tag = f"{btype}-{i}"
            tasks.append(
                asyncio.create_task(
                    worker_loop(tag, btype, i, metas, directed, lcd, container, mean_base, dry, fixed_mean=False),
                    name=tag,
                )
            )

    await asyncio.gather(*tasks)


def main() -> None:
    parser = argparse.ArgumentParser(description="LocalTerra Poisson swap + optional limit-order swarm")
    parser.add_argument(
        "--preflight-gas",
        action="store_true",
        help="Query test1 uluna and warn if below ~100k LUNC (GitLab #372); exit 0.",
    )
    parser.add_argument(
        "--worker",
        nargs=2,
        metavar=("TYPE", "REPLICA"),
        help="Run a single bot worker, e.g. --worker offer0 0 or --worker limit 0",
    )
    args = parser.parse_args()
    try:
        if args.preflight_gas:
            preflight_gas_balance()
            return
        if args.worker:
            btype, rid = args.worker[0], int(args.worker[1])
            asyncio.run(main_async_single(btype, rid))
        else:
            asyncio.run(main_async())
    except KeyboardInterrupt:
        print("Stopped.")


if __name__ == "__main__":
    main()
