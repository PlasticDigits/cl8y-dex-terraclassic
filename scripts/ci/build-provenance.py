#!/usr/bin/env python3
"""Record source, locked inputs, compiler/image identity and exact output hashes."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess


def sha256(path):
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def git(root, *arguments):
    return subprocess.check_output(["git", "-C", str(root), *arguments])


def receipt(root, source, lock, image, toolchain, artifacts):
    # Hash current tracked source bytes, including dirty edits. Outputs are never inputs.
    files = sorted(git(root, "ls-files", "--cached", "--others", "--exclude-standard", "-z", "--", source).decode().split("\0"))
    inputs = {}
    for filename in filter(None, files):
        path = Path(filename)
        if any(part in {"target", "artifacts", "node_modules", "dist"} for part in path.parts):
            continue
        if path.name == "wasm-checksums.txt":
            continue
        inputs[filename] = sha256(root / path)
    # Shared executable build recipes are inputs even though they live outside
    # the crate. Changing a compiler invocation must change the source digest.
    recipes = ["scripts/ci/build-provenance.py", "scripts/ci/build-indexer-release.sh",
               "scripts/ci/image-pins.json", ".dockerignore",
               "docker/indexer/Dockerfile", "docker/frontend/Dockerfile"]
    for filename in recipes:
        if (root / filename).is_file():
            inputs[filename] = sha256(root / filename)
    source_digest = hashlib.sha256(json.dumps(inputs, sort_keys=True).encode()).hexdigest()
    return {
        "schema": "cl8y-build-provenance-v1",
        "source_directory": source,
        "source_commit": git(root, "rev-parse", "HEAD").decode().strip(),
        "source_dirty": bool(git(root, "status", "--porcelain", "--", *inputs)),
        "source_sha256": source_digest,
        "source_inputs": inputs,
        "lockfile": {"path": lock, "sha256": sha256(root / lock)},
        "builder_image": image,
        "toolchain": toolchain,
        "cache_policy": "fresh-executable-target",
        "artifacts": {str(path.resolve().relative_to(root.resolve())): sha256(path)
                      for path in sorted(artifacts)},
    }


def verify(root, recorded, expected_image=None, inputs_only=False, required_artifacts=()):
    if recorded.get("schema") != "cl8y-build-provenance-v1":
        raise ValueError("unsupported provenance schema")
    if not inputs_only and not recorded.get("artifacts"):
        raise ValueError("output receipt has no artifact hashes")
    for path in required_artifacts:
        key = str(path.resolve().relative_to(root.resolve()))
        if key not in recorded["artifacts"]:
            raise ValueError(f"receipt omits required artifact: {key}")
    image = recorded["builder_image"]
    if expected_image is not None and image != expected_image:
        raise ValueError("builder image does not match the independently selected image")
    if image != "local-host" and not re.fullmatch(r"[\w./-]+:[\w.-]+@sha256:[0-9a-f]{64}", image):
        raise ValueError("builder image is not pinned by a manifest digest")
    paths = [root / path for path in recorded["artifacts"]]
    # A receipt can only refer to files inside this checkout.
    for path in [*paths, root / recorded["lockfile"]["path"],
                 root / recorded["source_directory"]]:
        path.resolve().relative_to(root.resolve())
    actual = receipt(root, recorded["source_directory"], recorded["lockfile"]["path"],
                     image, recorded["toolchain"], paths)
    for field in ["source_commit", "source_sha256", "source_inputs", "lockfile", "artifacts"]:
        if actual[field] != recorded[field]:
            raise ValueError(f"provenance mismatch: {field}")


def compare(recorded, rebuilt):
    # A separate cold build is required; this compares its evidence, it does not
    # authenticate either unsigned receipt or establish who ran the compiler.
    for field in ["source_commit", "source_sha256", "lockfile", "builder_image",
                  "toolchain", "artifacts"]:
        if recorded[field] != rebuilt[field]:
            raise ValueError(f"independent rebuild mismatch: {field}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--verify", type=Path)
    parser.add_argument("--compare", type=Path)
    parser.add_argument("--expected-image")
    parser.add_argument("--capture-inputs", action="store_true")
    parser.add_argument("--inputs-only", action="store_true")
    parser.add_argument("--require-artifact", type=Path, action="append", default=[])
    parser.add_argument("--source")
    parser.add_argument("--lock")
    parser.add_argument("--image")
    parser.add_argument("--toolchain")
    parser.add_argument("--output", type=Path)
    parser.add_argument("--artifact", type=Path, action="append")
    args = parser.parse_args()
    if args.verify:
        recorded = json.loads(args.verify.read_text())
        verify(args.root, recorded, args.expected_image, args.inputs_only, args.require_artifact)
        if args.compare:
            rebuilt = json.loads(args.compare.read_text())
            verify(args.root, rebuilt, args.expected_image, args.inputs_only, args.require_artifact)
            compare(recorded, rebuilt)
        print("PASS: receipt source, lockfile, image selection and output hashes verified" +
              ("; independent rebuild hashes match" if args.compare else ""))
        return
    if not all([args.source, args.lock, args.image, args.toolchain, args.output]) or (
        not args.capture_inputs and not args.artifact
    ):
        parser.error("generation requires --source, --lock, --image, --toolchain, --output and --artifact")
    args.output.write_text(json.dumps(receipt(
        args.root, args.source, args.lock, args.image, args.toolchain, args.artifact or []
    ), sort_keys=True, indent=2) + "\n")


if __name__ == "__main__":
    main()
