#!/usr/bin/env python3
"""Read-only LCD inventory and single-broadcast terrad signing transport."""
from __future__ import annotations

import argparse
import base64
import fcntl
import getpass
import gzip
import hashlib
import json
import os
import subprocess
import sys
import time
from pathlib import Path
from urllib.parse import quote
from urllib.error import HTTPError, URLError
from urllib.request import Request, urlopen

from route_tax_rollout import Artifact, DEFAULTS, SPECS, Rollout, RolloutError, require, validate_signers, write_json


KEY_ADDRESSES = {
    "cl8ydeploy": "terra1hu4zggf3f8yw6jw3rxrjxn2drwad675gq5k2lv",
    "cl8y2_admin": "terra1xsecn4snv94ezcez0z3vq8an9j4h4kxxcydp8l",
}


def redact(value, secret):
    return value.replace(secret, "[redacted]") if secret else value


class TerradChain:
    def __init__(self, args):
        self.args = args
        self.work = Path(args.state_dir) / "transactions"
        self.password = ""
        self.unlocked = False
        self.public_keys = {}
        self.roles = {"store": args.store_key, "gov": args.multisig_key}
        self.roles.update({key: key for key in args.admin_key})
        self.last_read = 0.0

    def get(self, path, rpc=False):
        delay = max(0, 0.12 - (time.monotonic() - self.last_read))
        if delay:
            time.sleep(delay)
        self.last_read = time.monotonic()
        url = (self.args.node if rpc else self.args.lcd).rstrip("/") + path
        request = Request(url, headers={"User-Agent": "CL8Y-route-tax-rollout/1"})
        for attempt in range(3):
            try:
                with urlopen(request, timeout=15) as response:
                    return json.load(response)
            except HTTPError as error:
                if error.code not in (429, 502, 503, 504) or attempt == 2:
                    raise RolloutError(f"Read-only endpoint returned HTTP {error.code}") from error
            except (URLError, TimeoutError) as error:
                if attempt == 2:
                    raise RolloutError("Read-only endpoint unavailable") from error
            time.sleep(attempt + 1)

    def network(self):
        rpc = self.get("/status", rpc=True)["result"]["node_info"]["network"]
        lcd = self.get("/cosmos/base/tendermint/v1beta1/blocks/latest")["block"]["header"]["chain_id"]
        require(rpc == lcd, "RPC and LCD serve different chains")
        return rpc

    def info(self, address):
        return self.get(f"/cosmwasm/wasm/v1/contract/{address}")["contract_info"]

    def code_hash(self, code):
        return self.get(f"/cosmwasm/wasm/v1/code/{code}")["code_info"]["data_hash"].lower()

    def version(self, address):
        key = quote(base64.b64encode(b"contract_info").decode(), safe="")
        raw = self.get(f"/cosmwasm/wasm/v1/contract/{address}/raw/{key}")["data"]
        require(raw, "Missing contract cw2 identity")
        return json.loads(base64.b64decode(raw))

    def smart(self, address, msg):
        encoded = quote(base64.b64encode(json.dumps(msg).encode()).decode(), safe="")
        value = self.get(f"/cosmwasm/wasm/v1/contract/{address}/smart/{encoded}")["data"]
        return json.loads(base64.b64decode(value)) if isinstance(value, str) else value

    def contracts(self, code, limit):
        values, cursor = [], ""
        for _ in range((limit + 29) // 30 + 1):
            path = f"/cosmwasm/wasm/v1/code/{code}/contracts?pagination.limit=30"
            if cursor:
                path += "&pagination.key=" + quote(cursor, safe="")
            page = self.get(path)
            values.extend(page["contracts"])
            require(len(values) <= limit and len(set(values)) == len(values), "Contract inventory exceeds bound or repeats")
            next_key = page.get("pagination", {}).get("next_key")
            if not next_key:
                return values
            require(next_key != cursor, "Code-instance cursor stalled")
            cursor = next_key
        raise RolloutError("Unbounded code-instance pagination")

    def command(self, parts, timeout=180):
        # Password travels only over stdin, never argv, diagnostics or journal.
        try:
            result = subprocess.run([self.args.terrad, *parts], input=(self.password + "\n") * 64 if self.password else None,
                                    capture_output=True, text=True, timeout=timeout)
        except subprocess.TimeoutExpired as error:
            # Captured output can contain a password echoed by a failed wrapper.
            raise RolloutError("terrad command timed out; reconcile any saved transaction before resuming") from error
        if result.returncode:
            raise RolloutError(redact(result.stderr, self.password)[:1200])
        # Keep successful SDK JSON/TxRaw internal and byte-exact. A password such
        # as "terra" may also occur in a legitimate public address; replacing it
        # here corrupts the signed data. Only diagnostics are printed/redacted.
        return result.stdout

    def flags(self):
        return ["--chain-id", self.args.chain_id, "--node", self.args.node,
                "--home", self.args.home, "--keyring-backend", self.args.keyring_backend]

    def unlock(self):
        if self.unlocked:
            return
        if self.args.keyring_backend == "file":
            self.password = os.environ.get("TERRAD_HOST_KEYRING_PASS", "")
            if not self.password:
                require(sys.stdin.isatty(), "Encrypted keyring requires an operator TTY; never supply a passphrase in chat")
                self.password = getpass.getpass("terrad keyring passphrase (used for this run only): ")
            require(self.password, "Empty keyring passphrase")
        records = {}
        for key in set([self.args.store_key, self.args.multisig_key, self.args.signer1, self.args.signer2, *self.args.admin_key]):
            records[key] = json.loads(self.command(["keys", "show", key, "--home", self.args.home,
                "--keyring-backend", self.args.keyring_backend, "--output", "json"]))
        require(records[self.args.store_key]["address"] == self.args.store_address, "Store key address mismatch")
        require(records[self.args.multisig_key]["address"] == self.args.governance, "Governance multisig address mismatch")

        def public(record):
            value = record["pubkey"]
            return json.loads(value) if isinstance(value, str) else value

        validate_signers(public(records[self.args.multisig_key]),
                         [public(records[self.args.signer1]), public(records[self.args.signer2])])
        for key in self.args.admin_key:
            require(records[key]["address"] == KEY_ADDRESSES.get(key, ""), "EOA admin key address mismatch")
        self.public_keys = {key: public(record) for key, record in records.items()}
        self.unlocked = True

    def _signed_proof(self, unsigned, signed, key, sequence, multisig):
        """Signing may add signer information and signatures, never change spend."""
        require(signed.get("body") == unsigned["body"], "Signed transaction body differs from approved action")
        auth = signed.get("auth_info", {})
        require({k: v for k, v in auth.items() if k != "signer_infos"} ==
                {k: v for k, v in unsigned["auth_info"].items() if k != "signer_infos"},
                "Signed transaction authorization/fee differs from approved action")
        signers = auth.get("signer_infos", [])
        require(len(signers) == 1 and str(signers[0].get("sequence")) == sequence,
                "Signed transaction signer sequence/count mismatch")
        require(signers[0].get("public_key") == self.public_keys[key], "Signed transaction public key mismatch")
        signatures = signed.get("signatures", [])
        require(len(signatures) == 1 and isinstance(signatures[0], str) and signatures[0],
                "Signed transaction signature count mismatch")
        if multisig:
            mode = signers[0].get("mode_info", {}).get("multi", {})
            require(len(mode.get("mode_infos", [])) == 2 and
                    all(m.get("single", {}).get("mode") == "SIGN_MODE_LEGACY_AMINO_JSON" for m in mode["mode_infos"]),
                    "Signed multisig must contain two legacy amino signer modes")
            keys = self.public_keys[key]["public_keys"]
            selected = [self.public_keys[name] for name in (self.args.signer1, self.args.signer2)]
            expected = bytes([sum(1 << (7 - index) for index, public in enumerate(keys) if public in selected)])
            bitarray = mode.get("bitarray", {})
            require(int(bitarray.get("extra_bits_stored", 0)) == 3 and
                    base64.b64decode(bitarray.get("elems", ""), validate=True) == expected,
                    "Signed multisig signer membership bits mismatch")
        else:
            require(signers[0].get("mode_info", {}).get("single", {}).get("mode") in
                    ("SIGN_MODE_DIRECT", "SIGN_MODE_LEGACY_AMINO_JSON"), "Unsupported signed transaction mode")

    def _account(self, address):
        value = self.get(f"/cosmos/auth/v1beta1/accounts/{address}")["account"]
        while "base_account" in value or "base_vesting_account" in value:
            value = value.get("base_account", value.get("base_vesting_account"))
        require(value["address"] == address, "Signer account address mismatch")
        return str(value["account_number"]), str(value["sequence"])

    def receipt(self, tx_hash):
        try:
            value = self.get(f"/cosmos/tx/v1beta1/txs/{tx_hash}")["tx_response"]
        except RolloutError as error:
            if "HTTP 404" in str(error):
                return None
            raise
        require(value.get("txhash") == tx_hash, "Receipt hash mismatch")
        if "code" in value and str(value["code"]) == "0":
            events = value.get("events") or [e for log in value.get("logs", []) for e in log["events"]]
            ids = [a["value"] for e in events if e["type"] == "store_code"
                   for a in e["attributes"] if a["key"] == "code_id"]
            if ids:
                require(len(ids) == 1, "Ambiguous store-code receipt")
                value["code_id"] = int(ids[0])
        return {k: value[k] for k in ("txhash", "code", "height", "gas_used", "code_id") if k in value}

    def _wait(self, tx_hash):
        for _ in range(60):
            receipt = self.receipt(tx_hash)
            if receipt:
                Rollout._receipt_ok(receipt)
                return receipt
            time.sleep(2)
        raise RolloutError(f"Inclusion unknown for {tx_hash}; resume with the saved journal, do not re-sign")

    def _encoded_hash(self, path):
        encoded = self.command(["tx", "encode", str(path), "--home", self.args.home]).strip()
        return hashlib.sha256(base64.b64decode(encoded, validate=True)).hexdigest().upper()

    def validate_saved(self, action, entry):
        """Verify resume data without unlocking the keyring or trusting fee edits."""
        signed = json.loads(Path(entry["signed_path"]).read_text())
        require(self._encoded_hash(entry["signed_path"]) == entry["tx_hash"], "Saved signed transaction changed")
        messages = signed["body"]["messages"]
        require(len(messages) == 1, "Saved transaction message count changed")
        msg = messages[0]
        types = {"store": "MsgStoreCode", "migrate": "MsgMigrateContract", "execute": "MsgExecuteContract"}
        signer = self.args.governance if action["role"] == "gov" else (
            self.args.store_address if action["role"] == "store" else KEY_ADDRESSES[self.roles[action["role"]]])
        require(msg.get("@type") == "/cosmwasm.wasm.v1." + types[action["op"]]
                and msg.get("sender") == signer and not msg.get("funds"), "Saved SDK operation changed")
        if action["op"] == "store":
            bytecode = base64.b64decode(msg["wasm_byte_code"])
            if bytecode.startswith(b"\x1f\x8b"):
                bytecode = gzip.decompress(bytecode)
            require(bytecode == Path(action["path"]).read_bytes(), "Saved store artifact changed")
        else:
            require(msg["contract"] == action["contract"] and msg["msg"] == action["msg"], "Saved operation changed")
            if action["op"] == "migrate":
                require(int(msg["code_id"]) == action["code_id"], "Saved migrate code changed")
        fee = signed["auth_info"]["fee"]
        require(0 < int(fee["gas_limit"]) <= self.args.max_gas and len(fee["amount"]) == 1,
                "Saved gas/fee envelope changed")
        require(fee["amount"][0]["denom"] == "uluna" and int(fee["amount"][0]["amount"]) == entry["fee_uluna"],
                "Saved fee accounting differs from signed transaction")

    def rebroadcast(self, path, expected_hash):
        require(self.network() == self.args.chain_id, "Rebroadcast chain mismatch")
        require(self._encoded_hash(path) == expected_hash, "Saved signed bytes changed")
        existing = self.receipt(expected_hash)
        if existing:
            return existing
        # A single deliberate replay of identical TxRaw has the same hash and cannot
        # become a new nonce/new store. Never retry by generating another transaction.
        output = json.loads(self.command(["tx", "broadcast", str(path), *self.flags(),
            "--broadcast-mode", "sync", "--output", "json"]))
        require(output.get("txhash") == expected_hash, "Broadcast hash mismatch")
        require("code" in output and str(output["code"]) == "0", "Broadcast rejected; reconcile saved transaction")
        return self._wait(expected_hash)

    def transaction(self, action, on_signed):
        self.unlock()
        require(self.network() == self.args.chain_id, "Signing chain mismatch")
        key = self.roles[action["role"]]
        signer = self.args.governance if action["role"] == "gov" else (
            self.args.store_address if action["role"] == "store" else KEY_ADDRESSES[key])
        account, sequence = self._account(signer)
        if action["op"] == "store":
            parts = ["wasm", "store", action["path"]]
        elif action["op"] == "migrate":
            parts = ["wasm", "migrate", action["contract"], str(action["code_id"]), json.dumps(action["msg"])]
        else:
            require(action["op"] == "execute", "Unsupported transaction operation")
            parts = ["wasm", "execute", action["contract"], json.dumps(action["msg"])]
        unsigned = json.loads(self.command(["tx", *parts, "--from", key, "--generate-only", *self.flags(),
            "--gas", "auto", "--gas-adjustment", str(self.args.gas_adjustment), "--gas-prices", self.args.gas_prices,
            "--output", "json"]))
        fee = unsigned["auth_info"]["fee"]
        require(0 < int(fee["gas_limit"]) <= self.args.max_gas, "Per-transaction gas bound exceeded")
        require(len(fee["amount"]) == 1 and fee["amount"][0]["denom"] == "uluna", "Unexpected fee denomination")
        amount = int(fee["amount"][0]["amount"])
        require(amount > 0, "Invalid fee amount")
        require(len(unsigned["body"]["messages"]) == 1, "Unexpected generated message count")
        body = unsigned["body"]
        require(set(body) <= {"messages", "memo", "timeout_height", "unordered", "timeout_timestamp",
                             "extension_options", "non_critical_extension_options"}
                and not body.get("memo") and str(body.get("timeout_height", 0)) == "0"
                and body.get("unordered", False) is False
                and body.get("timeout_timestamp") in (None, "", "0001-01-01T00:00:00Z")
                and not body.get("extension_options") and not body.get("non_critical_extension_options"),
                "Generated transaction contains unapproved memo/extension/timeout")
        auth = unsigned["auth_info"]
        require(set(auth) <= {"signer_infos", "fee", "tip"} and not auth.get("tip")
                and not fee.get("payer") and not fee.get("granter"), "Generated fee payer/granter/tip is not approved")
        msg = unsigned["body"]["messages"][0]
        types = {"store": "MsgStoreCode", "migrate": "MsgMigrateContract", "execute": "MsgExecuteContract"}
        require(msg.get("@type") == "/cosmwasm.wasm.v1." + types[action["op"]],
                "Generated SDK message type differs from approved operation")
        require(msg["sender"] == signer and not msg.get("funds"), "Generated signer/funds differ from approved action")
        if action["op"] == "store":
            bytecode = base64.b64decode(msg["wasm_byte_code"])
            if bytecode.startswith(b"\x1f\x8b"):
                bytecode = gzip.decompress(bytecode)
            require(bytecode == Path(action["path"]).read_bytes(), "Generated store bytes differ from artifact")
        else:
            require(msg["contract"] == action["contract"] and msg["msg"] == action["msg"], "Generated operation differs from approved action")
            if action["op"] == "migrate":
                require(int(msg["code_id"]) == action["code_id"], "Generated migrate code differs")
        self.work.mkdir(parents=True, exist_ok=True, mode=0o700)
        self.work.chmod(0o700)
        folder = self.work / str(time.time_ns())
        folder.mkdir(mode=0o700)
        unsigned_path, signed_path = folder / "unsigned.json", folder / "signed.json"
        write_json(unsigned_path, unsigned)
        offline = ["--offline", "--account-number", account, "--sequence", sequence]
        if action["role"] == "gov":
            signatures = []
            for index, member in enumerate((self.args.signer1, self.args.signer2)):
                signature = json.loads(self.command(["tx", "sign", str(unsigned_path), "--from", member,
                    "--multisig", signer, "--sign-mode", "amino-json", *offline, *self.flags(), "--output", "json"]))
                path = folder / f"signature{index}.json"
                write_json(path, signature)
                signatures.append(str(path))
            signed = json.loads(self.command(["tx", "multisign", str(unsigned_path), key, *signatures,
                *offline, *self.flags(), "--output", "json"]))
        else:
            signed = json.loads(self.command(["tx", "sign", str(unsigned_path), "--from", key,
                *offline, *self.flags(), "--output", "json"]))
        self._signed_proof(unsigned, signed, key, sequence, action["role"] == "gov")
        write_json(signed_path, signed)
        tx_hash = self._encoded_hash(signed_path)
        on_signed(tx_hash, str(signed_path), amount)
        print(f"Broadcast {action['op']} via {key}: {tx_hash}", flush=True)
        return self.rebroadcast(signed_path, tx_hash)


def parser():
    root = Path(__file__).resolve().parents[2]
    p = argparse.ArgumentParser(description=__doc__)
    mode = p.add_mutually_exclusive_group()
    for name in ("execute", "verify", "finalize"):
        mode.add_argument("--" + name, action="store_true")
    p.add_argument("--state-dir", default=str(Path.home() / ".local/state/cl8y-dex/route-tax-1381"))
    p.add_argument("--artifacts", type=Path, default=root / "smartcontracts/artifacts")
    p.add_argument("--checksums", type=Path)
    for name in ("factory", "router", "launcher", "cmm", "governance", "alpha", "chain-id"):
        p.add_argument("--" + name, default=DEFAULTS[name.replace("-", "_")])
    p.add_argument("--token", action="append", default=[])
    p.add_argument("--tax-free-code-id", type=int, action="append")
    p.add_argument("--node", default=os.environ.get("TERRAD_HOST_NODE", "https://terra-classic-rpc.publicnode.com:443"))
    p.add_argument("--lcd", default=os.environ.get("TERRA_LCD_URL", "https://terra-classic-lcd.publicnode.com"))
    p.add_argument("--terrad", default="terrad")
    p.add_argument("--home", default=os.environ.get("TERRAD_HOST_HOME", str(Path.home() / ".terra")))
    p.add_argument("--keyring-backend", choices=("file", "os", "test"), default=os.environ.get("TERRAD_HOST_KEYRING_BACKEND", "file"))
    p.add_argument("--store-key", default="cl8ydeploy")
    p.add_argument("--store-address", default=KEY_ADDRESSES["cl8ydeploy"])
    p.add_argument("--multisig-key", default="multisig_2of3")
    p.add_argument("--signer1", default="multisig1")
    p.add_argument("--signer2", default="multisig2")
    p.add_argument("--admin-key", action="append", default=[])
    p.add_argument("--gas-prices", default="28.325uluna")
    p.add_argument("--gas-adjustment", type=float, default=1.4)
    p.add_argument("--max-gas", type=int, default=20_000_000)
    p.add_argument("--max-total-fee-uluna", type=int, default=DEFAULTS["max_total_fee_uluna"])
    p.add_argument("--rebroadcast-pending", action="store_true")
    for role in ("cmm", "launcher", "autolp"):
        p.add_argument("--" + role + "-wasm", type=Path)
        p.add_argument("--" + role + "-sha256")
    return p


def main(argv=None):
    args = parser().parse_args(argv)
    lock = None
    chain = None
    try:
        require(1 <= args.gas_adjustment <= 3 and 0 < args.max_gas <= 50_000_000 and args.max_total_fee_uluna > 0,
                "Invalid gas/fee bounds")
        require(args.chain_id != "columbus-5" or args.keyring_backend != "test", "Production cannot use a test keyring")
        checksums = {}
        for line in (args.checksums or args.artifacts / "checksums.txt").read_text().splitlines():
            fields = line.split()
            if len(fields) == 2:
                checksums[fields[1].lstrip("*")] = fields[0].lower()
        artifacts = {role: Artifact.load(role, args.artifacts / SPECS[role][2], checksums.get(SPECS[role][2]))
                     for role in ("factory", "pair", "router", "token")}
        for role in ("cmm", "launcher", "autolp"):
            path = getattr(args, role + "_wasm")
            if path:
                artifacts[role] = Artifact.load(role, path, getattr(args, role + "_sha256"))
        config = {name: getattr(args, name) for name in ("factory", "router", "launcher", "cmm", "governance", "alpha", "chain_id")}
        config.update(max_total_fee_uluna=args.max_total_fee_uluna,
                      tax_free_code_ids=args.tax_free_code_id or DEFAULTS["tax_free_code_ids"],
                      eoa_admins={KEY_ADDRESSES[key]: key for key in args.admin_key},
                      transport={name: str(getattr(args, name)) for name in ("node", "lcd", "home", "keyring_backend", "store_key", "store_address", "multisig_key", "signer1", "signer2", "gas_prices", "gas_adjustment", "max_gas")})
        chain = TerradChain(args)
        state_dir = Path(args.state_dir).resolve()
        repo = Path(__file__).resolve().parents[2]
        require(not state_dir.is_relative_to(repo), "Rollout receipts must be outside the repository")
        state_dir.mkdir(parents=True, exist_ok=True, mode=0o700)
        state_dir.chmod(0o700)
        lock_fd = os.open(state_dir / "rollout.lock", os.O_RDWR | os.O_CREAT, 0o600)
        os.fchmod(lock_fd, 0o600)
        lock = os.fdopen(lock_fd, "w")
        try:
            fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError as error:
            raise RolloutError("Another rollout process owns this state directory") from error
        engine = Rollout(chain, artifacts, config, Path(args.state_dir) / "journal.json", args.token)
        state = engine.prepare()
        if args.execute:
            state = engine.execute(args.rebroadcast_pending)
        elif args.verify:
            print(json.dumps(engine.verify(), indent=2))
        elif args.finalize:
            state = engine.finalize(args.rebroadcast_pending)
        if state.get("status") in ("contracts_ready_paused", "complete") and state.get("codes", {}).get("token"):
            engine.verify()
            code = state["codes"]["token"]
            tax_ids = sorted({code, *[int(chain.info(t["address"])["code_id"]) for t in state["inventory"]["tokens"]]})
            handoff = "\n".join([f"TRADING_ROUTE_TAX_CODE_IDS={','.join(map(str,tax_ids))}",
                f"TRADING_ROUTE_TAX_DATA_HASHES={artifacts['token'].sha256}",
                f"TRADING_TAX_FREE_CODE_IDS={','.join(map(str,config['tax_free_code_ids']))}",
                f"COMMUNITY_TAX_CODE_ID={code}", f"COMMUNITY_TAX_OPTION2_CODE_IDS={code}",
                f"VITE_COMMUNITY_TAX_CODE_ID={code}", f"ROUTER_ADDRESS={args.router}",
                f"VITE_ROUTER_ADDRESS={args.router}"]) + "\n"
            if "autolp" in artifacts:
                handoff += f"COMMUNITY_AUTOLP_CODE_ID={state['codes']['autolp']}\n"
            handoff_path = Path(args.state_dir, "deployment.env")
            fd = os.open(handoff_path, os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o600)
            os.fchmod(fd, 0o600)
            with os.fdopen(fd, "w") as stream:
                stream.write(handoff)
        else:
            Path(args.state_dir, "deployment.env").unlink(missing_ok=True)
        print(json.dumps({"status": state["status"], "state_dir": args.state_dir,
                          "tokens": [t["address"] for t in state["inventory"]["tokens"]],
                          "affected_pairs": state["inventory"]["affected_pairs"],
                          "codes": state.get("codes"), "production_execution": "UNVERIFIED"}, indent=2))
        return 0
    except (RolloutError, OSError, ValueError, KeyError, subprocess.TimeoutExpired) as error:
        secret = getattr(chain, "password", "")
        print("Rollout stopped: " + redact(str(error), secret), file=sys.stderr)
        return 1
    finally:
        if lock is not None:
            lock.close()


if __name__ == "__main__":
    raise SystemExit(main())
