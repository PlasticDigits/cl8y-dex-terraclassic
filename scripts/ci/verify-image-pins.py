#!/usr/bin/env python3
"""Validate recorded registry manifest pins; --registry also checks remote bytes."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import urllib.request

ROOT = Path(__file__).resolve().parents[2]
ACCEPT = ",".join([
    "application/vnd.oci.image.index.v1+json",
    "application/vnd.docker.distribution.manifest.list.v2+json",
    "application/vnd.oci.image.manifest.v1+json",
    "application/vnd.docker.distribution.manifest.v2+json",
])


def declared_images(path):
    content = path.read_text()
    if path.name == "Dockerfile":
        images = re.findall(r"^FROM\s+(?:--platform=\S+\s+)?(\S+)", content, re.MULTILINE)
        if not images:
            raise ValueError(f"Dockerfile has no recognized base image: {path}")
        return images
    if path.name in {"optimize.sh", "optimize-project.sh"}:
        images = re.findall(r'^IMAGE="([^"]+)"', content, re.MULTILINE)
        if len(images) != 1:
            raise ValueError(f"optimizer must declare exactly one pinned IMAGE: {path}")
        return images
    if path.suffix == ".sh":
        images = re.findall(r'GITLEAKS_IMAGE="\$\{GITLEAKS_IMAGE:-([^}]+)\}"', content)
        if len(images) != 1:
            raise ValueError(f"scanner must declare exactly one pinned image default: {path}")
        return images
    lines = content.splitlines()
    variables = dict(re.findall(r'^\s*([A-Z_]+):\s*["\']?([^\s"\']+)', content, re.MULTILINE))
    images = []
    for index, line in enumerate(lines):
        match = re.match(r"^\s*image:\s*(.*)", line)
        if match:
            value = match[1].split(" #", 1)[0].strip().strip("\"'")
            if not value:
                # GitLab's image mapping carries its reference under name.
                following = re.match(r"^\s*name:\s*(\S+)", lines[index + 1])
                if not following:
                    raise ValueError(f"unrecognized image mapping: {path}:{index + 1}")
                value = following[1].strip("\"'")
            if value.startswith("$"):
                value = variables.get(value.removeprefix("$").strip("{}"), value)
            images.append(value)
        if re.match(r"^\s*services:\s*$", line) and path.name == ".gitlab-ci.yml":
            indentation = len(line) - len(line.lstrip())
            for nested in lines[index + 1:]:
                if nested.strip() and len(nested) - len(nested.lstrip()) <= indentation:
                    break
                service = re.match(r"^\s*-\s*(?:name:\s*)?(\S+)", nested)
                if service:
                    images.append(service[1].strip("\"'"))
    return images


def validate_reference(reference, pins):
    match = re.fullmatch(r"([\w./-]+:[\w.-]+)@(sha256:[0-9a-f]{64})", reference)
    if not match:
        raise ValueError(f"floating or malformed image input: {reference}")
    image, digest = match.groups()
    if pins.get(image) != digest:
        raise ValueError(f"unrecorded image digest: {reference}")
    return image


def registry_digest(reference, digest):
    name = reference.rsplit(":", 1)[0]
    if name.startswith("ghcr.io/"):
        registry, repository = "ghcr.io", name.removeprefix("ghcr.io/")
        issuer, service = "https://ghcr.io/token", "ghcr.io"
    else:
        registry = "registry-1.docker.io"
        repository = name if "/" in name else "library/" + name
        issuer, service = "https://auth.docker.io/token", "registry.docker.io"
    with urllib.request.urlopen(
        f"{issuer}?service={service}&scope=repository:{repository}:pull", timeout=30
    ) as response:
        token = json.load(response)["token"]
    request = urllib.request.Request(
        f"https://{registry}/v2/{repository}/manifests/{digest}",
        headers={"Authorization": "Bearer " + token, "Accept": ACCEPT},
    )
    with urllib.request.urlopen(request, timeout=30) as response:
        actual = "sha256:" + hashlib.sha256(response.read()).hexdigest()
        if actual != digest or response.headers.get("Docker-Content-Digest", digest) != digest:
            raise ValueError(f"manifest hash mismatch: {reference}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--registry", action="store_true")
    args = parser.parse_args()
    pins = json.loads((ROOT / "scripts/ci/image-pins.json").read_text())
    files = [ROOT / ".gitlab-ci.yml", ROOT / ".woodpecker.yaml",
             ROOT / "smartcontracts/scripts/optimize.sh", ROOT / "scripts/ci/optimize-project.sh",
             ROOT / "scripts/ci/gitleaks-scan-tracked.sh", ROOT / "scripts/ci/verify-gitleaks.sh",
             *ROOT.glob("docker/*/Dockerfile"),
             *ROOT.glob("scripts/ci/trusted-policy/Dockerfile")]
    found = set()
    for path in files:
        for reference in declared_images(path):
            found.add(validate_reference(reference, pins))
    if args.registry:
        for image in sorted(found):
            registry_digest(image, pins[image])
    print(f"PASS: {len(found)} recorded immutable image inputs" +
          (" verified against registry bytes" if args.registry else ""))


if __name__ == "__main__":
    main()
