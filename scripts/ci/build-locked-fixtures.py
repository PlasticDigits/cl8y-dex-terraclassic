#!/usr/bin/env python3
"""Build external QA fixtures at exact source revisions in the DEX Cargo.lock."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess
import tempfile
import tomllib

ROOT = Path(__file__).resolve().parents[2]
FIXTURES = {
    "cw20": ("cw20-mintable", ".", ["cw20_mintable.wasm"]),
    "ustr": ("treasury", "contracts", ["treasury.wasm", "wrap_mapper.wasm"]),
}


def locked_source(lockfile, name):
    data = tomllib.loads(lockfile.read_text())
    sources = [package["source"] for package in data["package"] if package["name"] == name]
    if len(sources) != 1:
        raise ValueError(f"expected one locked source for {name}")
    match = re.fullmatch(r"git\+(https://[^?#]+)(?:\?[^#]+)?#([0-9a-f]{40})", sources[0])
    if not match:
        raise ValueError(f"fixture source is not a locked Git commit: {name}")
    return match.groups()


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def prepare_legacy_lock(path):
    # Cargo 1.81 in optimizer 0.16.1 cannot read format 4. This deterministic
    # format-only conversion preserves every locked package, revision/checksum.
    # Do not regenerate or resolve dependencies against current registry state.
    content = path.read_text()
    data = tomllib.loads(content)
    if data.get("version") == 4:
        converted = re.sub(r"^version = 4$", "version = 3", content, count=1, flags=re.MULTILINE)
        if tomllib.loads(converted)["package"] != data["package"]:
            raise ValueError("legacy lock conversion changed the dependency graph")
        path.write_text(converted)
        return "4-to-3"
    if data.get("version") != 3:
        raise ValueError("unsupported external fixture lock format")
    return "none"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("fixture", choices=FIXTURES)
    parser.add_argument("--artifacts", type=Path, required=True)
    parser.add_argument("--print-source", action="store_true")
    args = parser.parse_args()
    package, workspace, outputs = FIXTURES[args.fixture]
    url, revision = locked_source(ROOT / "smartcontracts/Cargo.lock", package)
    if args.print_source:
        print(f"{url}#{revision}")
        return
    args.artifacts.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="cl8y-locked-fixture-") as directory:
        checkout = Path(directory)
        subprocess.run(["git", "init", "-q", str(checkout)], check=True)
        subprocess.run(["git", "-C", str(checkout), "fetch", "--depth", "1", url, revision], check=True)
        subprocess.run(["git", "-C", str(checkout), "checkout", "--detach", revision], check=True)
        subprocess.run(["git", "-C", str(checkout), "submodule", "update", "--init", "--recursive"], check=True)
        project = checkout / workspace
        # Missing locks fail before executing any container. Never resolve current tags.
        source_lock_digest = digest(project / "Cargo.lock")
        conversion = prepare_legacy_lock(project / "Cargo.lock")
        lock_digest = digest(project / "Cargo.lock")
        subprocess.run(["bash", str(ROOT / "scripts/ci/optimize-project.sh"), str(project)], check=True)
        if digest(project / "Cargo.lock") != lock_digest:
            raise ValueError("fixture Cargo.lock changed during build")
        hashes = {}
        for filename in outputs:
            artifact = project / "artifacts" / filename
            data = artifact.read_bytes()
            (args.artifacts / filename).write_bytes(data)
            hashes[filename] = hashlib.sha256(data).hexdigest()
        pins = json.loads((ROOT / "scripts/ci/image-pins.json").read_text())
        image = "cosmwasm/workspace-optimizer:0.16.1"
        receipt = {
            "schema": "cl8y-external-fixture-v1", "source_repository": url,
            "source_commit": revision, "lock_sha256": lock_digest,
            "source_lock_sha256": source_lock_digest, "lock_format_conversion": conversion,
            "builder_image": image + "@" + pins[image],
            "toolchain": (project / "artifacts/toolchain.txt").read_text().strip(),
            "cache_policy": "fresh-executable-target", "artifacts": hashes,
        }
        (args.artifacts / f"{args.fixture}-fixture-provenance.json").write_text(
            json.dumps(receipt, sort_keys=True, indent=2) + "\n"
        )


if __name__ == "__main__":
    main()
