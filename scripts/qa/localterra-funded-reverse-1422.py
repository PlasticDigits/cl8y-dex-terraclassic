#!/usr/bin/env python3
"""Run the funded reverse regression on an isolated, unmodified 3M LocalTerra node.

Smart queries establish provider-limit compatibility. Test-only query-probe
transactions measure gas including transaction/wrapper overhead; they are not
identical to LCD query-meter readings. No production queries or state resets.
"""
import argparse
import base64
import importlib.util
import hashlib
import json
import re
import subprocess
import tomllib
from pathlib import Path

SPEC = importlib.util.spec_from_file_location("funded_fixture", Path(__file__).with_name("localterra-route-tax-1381.py"))
fixture = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(fixture)
QUERY_LIMIT = 3_000_000


def gas_failure(error):
    text = error.lower()
    return "out of gas" in text and re.search(r"gaswanted\s*:\s*3000000\b", text) is not None and "gasused" in text


def node_metadata(chain, args):
    config = tomllib.loads(chain.command("cat", args.node_home + "/config/app.toml"))
    actual_limit = int(config["wasm"]["query_gas_limit"])
    assert actual_limit == QUERY_LIMIT, f"Expected untouched 3M query limit, got {actual_limit}"
    inspected = json.loads(subprocess.check_output(["docker", "inspect", chain.container]))[0]
    compose = Path(__file__).resolve().parents[2] / "docker-compose.yml"
    pinned_image = re.search(r"image:\s*(\S*/localterra-cl8y@sha256:[0-9a-f]+)", compose.read_text())
    assert pinned_image and inspected["Config"]["Image"] == pinned_image.group(1), "Use the reviewed repository-pinned SDK53 image"
    version = chain.command("terrad", "version", "--long")
    assert "cosmos_sdk_version: v0.53." in version, "Expected SDK53 node"
    # Image identity pins the complete binary; publish concise runtime fields.
    # Avoid an irrelevant dependency dump being mistaken for access-token data.
    version_fields = {}
    for line in version.splitlines():
        key, separator, value = line.partition(":")
        if separator and key in {"app_name", "commit", "cosmos_sdk_version", "go",
                                "name", "version", "wasmvm_version", "build_tags"}:
            version_fields[key] = value.strip()
    return {"container": chain.container, "image": inspected["Config"]["Image"],
            "image_id": inspected["Image"], "terrad_version": version_fields,
            "terrad_version_dump_sha256": hashlib.sha256(version.encode()).hexdigest(),
            "wasm_query_gas_limit": actual_limit}


class QueryEvidence:
    def __init__(self, args, baseline=False):
        self.args = args
        self.baseline = baseline
        self.last_evidence = None
        self.last_save = None
        self.probe = None

    def setup(self, chain, evidence, save):
        self.last_evidence, self.last_save = evidence, save
        evidence["node"] = node_metadata(chain, self.args)
        evidence["gas_scope"] = "probe transaction including transaction and wrapper overhead; actual smart-query success separately recorded"
        for stored in evidence["codes"].values():
            info = chain.query("wasm", "code-info", str(stored["code_id"]))
            assert info["checksum"].lower() == stored["sha256"], "Stored fixture checksum mismatch"
            stored["onchain_sha256"] = info["checksum"].lower()
        evidence["contract_versions"] = {}
        for label, address in {"router": evidence["router"], "factory": evidence["factory"], **evidence["pairs"],
                               **evidence["tokens"]}.items():
            raw = chain.query("wasm", "contract-state", "raw", address, "contract_info", "--ascii")
            evidence["contract_versions"][label] = json.loads(base64.b64decode(raw["data"]))
        if not self.baseline:
            assert evidence["contract_versions"]["router"] == {
                "contract": "cl8y-dex-router", "version": "1.3.1"}, "Expected reviewed router release"
            for label in evidence["pairs"]:
                assert evidence["contract_versions"][label] == {
                    "contract": "cl8y-dex-pair", "version": "1.20.0"}, "Expected reviewed pair release"
        stored = chain.store(self.args.probe)
        info = chain.query("wasm", "code-info", str(stored["code_id"]))
        assert info["checksum"].lower() == stored["sha256"], "Stored probe checksum mismatch"
        stored["onchain_sha256"] = info["checksum"].lower()
        self.probe = chain.instantiate(stored["code_id"], {}, "issue1422-test-query-probe")
        evidence["probe_code"] = stored
        evidence["query_checks"] = []
        save()

    def query(self, chain, evidence, scenario, kind, address, message, save):
        row = {"scenario": scenario, "kind": kind, "contract": address,
               "message": message, "smart_status": "PENDING"}
        evidence["query_checks"].append(row)
        save()
        response, smart_error = None, None
        try:
            response = chain.smart(address, message)
            row.update(smart_status="PASS", response=response)
        except RuntimeError as error:
            smart_error = error
            row.update(smart_status="FAIL", error=str(error))
        save()  # Preserve the real provider failure before any measurement attempt.
        receipt = chain.execute(self.probe, {"contract": address,
            "query": base64.b64encode(json.dumps(message).encode()).decode()})
        status = chain.event(receipt, "wasm", "query_status")
        row["probe"] = {"txhash": receipt["txhash"], "height": receipt["height"],
                        "gas_used": int(receipt["gas_used"]), "query_status": status}
        if status == "error":
            row["probe"]["error"] = chain.event(receipt, "wasm", "query_error")
        save()
        if smart_error:
            raise smart_error
        assert status == "ok", row
        return response


def run_fixture(args, artifacts, destination, baseline=False):
    observer = QueryEvidence(args, baseline=baseline)
    command = ["--container", args.container, "--artifacts", str(artifacts),
               "--evidence-file", str(destination), "--isolated-owner"]
    if args.code_cache:
        command += ["--code-cache", str(args.code_cache)]
    if baseline:
        command += ["--scenario", "multi tax intermediates"]
    try:
        fixture.main(command, query_observer=observer.query, fixture_observer=observer.setup)
    except Exception as error:
        evidence = observer.last_evidence
        if evidence:
            evidence["status"] = "FAIL"
            evidence["failure"] = str(error)
            observer.last_save()
        # Baseline success means an independently deployed frozen old artifact
        # reproduced the named route's real 3M smart-query gas failure.
        failed = [row for row in (evidence or {}).get("query_checks", [])
                  if row["kind"] == "reverse" and row["smart_status"] == "FAIL"]
        if (baseline and failed and gas_failure(failed[-1]["error"])
                and str(error) == failed[-1]["error"] and "probe" in failed[-1]):
            evidence["baseline_regression"] = "REPRODUCED"
            observer.last_save()
            print("PASS frozen baseline reproduced reverse gas failure", flush=True)
            return
        raise
    if baseline:
        observer.last_evidence["baseline_regression"] = "NOT_REPRODUCED"
        observer.last_save()
        raise AssertionError("Frozen baseline unexpectedly passed; verify artifact provenance")
    evidence = observer.last_evidence
    assert len(evidence["checks"]) == 6, "All six funded scenarios are required"
    assert len(evidence["query_checks"]) == 18, "Each scenario requires forward/reverse/fresh validation"
    assert all(row["smart_status"] == "PASS" for row in evidence["query_checks"])
    assert all(row["probe"]["query_status"] == "ok" for row in evidence["query_checks"])
    observer.last_save()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--container", default="cl8y-dex-issue1422-localterra")
    parser.add_argument("--node-home", default="/home/terra/.terra")
    parser.add_argument("--artifacts", required=True, type=Path)
    parser.add_argument("--baseline-artifacts", type=Path)
    parser.add_argument("--probe", required=True, type=Path)
    parser.add_argument("--code-cache", type=Path)
    parser.add_argument("--evidence-file", required=True, type=Path)
    args = parser.parse_args()
    # Reject a relaxed limit before provisioning or broadcasting any fixture.
    chain = fixture.LocalTerra(args.container)
    node_metadata(chain, args)
    if args.baseline_artifacts:
        baseline = args.evidence_file.with_name(args.evidence_file.stem + "-baseline.json")
        run_fixture(args, args.baseline_artifacts, baseline, baseline=True)
    run_fixture(args, args.artifacts, args.evidence_file)


if __name__ == "__main__":
    main()
