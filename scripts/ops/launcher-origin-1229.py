#!/usr/bin/env python3
"""Use the existing governed rollout transport for the #1229 origin upgrade.

Default is read-only planning. No code IDs or custody secrets are embedded.
"""
import hashlib
import json
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/lib"))
import route_tax_rollout as rollout
import route_tax_rollout_cli as cli


def main():
    allowed = {"--execute", "--verify", "--finalize", "--check-artifacts"}
    if len(sys.argv) > 2 or any(arg not in allowed for arg in sys.argv[1:]):
        raise SystemExit("Usage: launcher-origin-1229.py [--check-artifacts|--execute|--verify|--finalize]")
    # Process-local release versions; historical release manifests stay immutable.
    for role, version in (("token", "1.5.0"), ("launcher", "1.1.0")):
        contract, _, filename = rollout.SPECS[role]
        rollout.SPECS[role] = (contract, version, filename)
    artifacts = ROOT / "smartcontracts/artifacts"
    launcher = artifacts / rollout.SPECS["launcher"][2]
    checksums = dict((name.lstrip("*"), digest) for digest, name in
                     (line.split() for line in (artifacts / "checksums.txt").read_text().splitlines()))
    launcher_hash = checksums[launcher.name]
    if hashlib.sha256(launcher.read_bytes()).hexdigest() != launcher_hash:
        raise SystemExit("Launcher artifact differs from reviewed checksum manifest")

    if sys.argv[1:] == ["--check-artifacts"]:
        loaded = {role: rollout.Artifact.load(role, artifacts / rollout.SPECS[role][2],
                                             checksums.get(rollout.SPECS[role][2]))
                  for role in ("factory", "pair", "router", "token", "launcher")}
        print(json.dumps({"status": "PASS", "scope": "local artifacts only; no chain reads or custody",
                          "artifacts": {role: {"version": item.version, "sha256": item.sha256}
                                        for role, item in loaded.items()}}, indent=2))
        return 0

    # A security-only release may not silently migrate the factory, router, pair
    # or treasury. The existing engine owns pauses, token migrations, F6 refresh,
    # exact code/checksum readbacks and immutable, resumable transaction receipts.
    original_inventory = rollout.Rollout.inventory

    def inventory(self):
        result = original_inventory(self)
        unchanged = [*(item for role, item in result["instances"].items()
                       if role != "launcher"), *result["pairs"]]
        rollout.require(not any(item["needs_migrate"] for item in unchanged),
                        "Unrelated contract upgrade required; prepare it separately")
        return result

    rollout.Rollout.inventory = inventory
    return cli.main([
        "--state-dir", str(Path.home() / ".local/state/cl8y-dex/launcher-origin-1229"),
        "--artifacts", str(artifacts), "--launcher-wasm", str(launcher),
        "--launcher-sha256", launcher_hash, "--admin-key", "cl8y2_admin",
        "--read-node", "https://rpc.terra-classic.hexxagon.io:443",
        "--execution-node", "https://rpc.terra-classic.hexxagon.io:443",
        *sys.argv[1:],
    ])


if __name__ == "__main__":
    raise SystemExit(main())
