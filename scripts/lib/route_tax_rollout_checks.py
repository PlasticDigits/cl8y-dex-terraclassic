"""Shared read-only trust checks for rollout and public route-tax probes."""
from __future__ import annotations

import re


def router_binding_error(config, router):
    """Legacy and funded tokens must retain the inventoried router address."""
    if not isinstance(config, dict) or config.get("router") != router:
        return "Token configured router mismatch; immediate rollout requires in-place migration"
    return None


def funded_capability_error(capability, router=None):
    """Capability is checked in addition to reviewed code identity, never instead."""
    if not isinstance(capability, dict) or type(capability.get("funding_version")) is not int or capability["funding_version"] != 1:
        return "Router/token does not advertise funding version 1"
    if router is not None and capability.get("router") != router:
        return "Token funding/router capability mismatch"
    return None


def reviewed_code_error(code_id, data_hash, reviewed_codes):
    """Bind actual deployed code IDs to reviewed wasm hashes, ignoring CW2 text."""
    if type(code_id) is bool or not str(code_id).isdecimal() or int(code_id) <= 0:
        return "Malformed live code ID"
    expected = reviewed_codes.get(int(code_id))
    if not expected:
        return "Live code ID has no reviewed artifact hash"
    if not isinstance(data_hash, str) or not re.fullmatch(r"[0-9a-fA-F]{64}", data_hash) or data_hash.lower() != expected:
        return "Live code ID/data hash differs from reviewed artifact"
    return None


def router_settlement_error(config):
    if not isinstance(config, dict) or type(config.get("settlement_version")) is not int or config["settlement_version"] != 1:
        return "Router does not advertise single-use settlement version 1"
    route_id = config.get("last_route_id")
    if type(route_id) is not int or not 0 <= route_id <= 2**64 - 1:
        return "Malformed router monotonic route ID"
    return None


def router_settings(config):
    """Protocol metadata/counters may change; operator-selected settings may not."""
    return {key: value for key, value in config.items()
            if key not in ("funding_version", "settlement_version", "last_route_id")}
