#!/usr/bin/env python3
"""Synthetic-only regression checks for sanitized collector inventory."""
import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location(
    "inventory", Path(__file__).with_name("logs-collector-inventory.py")
)
inventory = importlib.util.module_from_spec(spec)
spec.loader.exec_module(inventory)


class CollectorInventoryTests(unittest.TestCase):
    def test_cl8y_assignments_are_found_without_values(self):
        body = """# CL8Y_LOGGING_PASSWORD=comment
CL8Y_LOGGING_USERNAME=synthetic-user
export CL8Y_LOGGING_PASSWORD='synthetic-password'
  LOKI_PUSH_URL = https://example.invalid
UNKNOWN_SECRET=synthetic-other
"""
        self.assertEqual(inventory.credential_keys(body), [
            "CL8Y_LOGGING_PASSWORD", "CL8Y_LOGGING_USERNAME", "LOKI_PUSH_URL"
        ])

    def test_container_summary_never_exports_values(self):
        result = inventory.container_summary([{
            "Config": {"Env": ["CL8Y_LOGGING_PASSWORD=synthetic-password"],
                       "Labels": {"com.docker.compose.service": "alloy"}},
            "State": {"Running": True},
        }])
        self.assertEqual(result, [{"compose_service": "alloy",
                                  "credential_keys": ["CL8Y_LOGGING_PASSWORD"],
                                  "running": True}])


if __name__ == "__main__":
    unittest.main()
