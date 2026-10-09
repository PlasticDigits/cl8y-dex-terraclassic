#!/usr/bin/env python3
"""Synthetic privacy, read-only, redirect and endpoint-budget regression controls."""
import importlib.util
import json
import os
import time
import unittest
from pathlib import Path
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location('probe', Path(__file__).with_name('probe-issue-1453.py'))
PROBE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(PROBE)
SECRET = 'synthetic_test_secret_never_publish'


class ProbeTests(unittest.TestCase):
    def test_configuration_is_allowlisted_redacted_and_provider_count_bounded(self):
        def read(url, token=None):
            if url.endswith('/envs'):
                self.assertEqual(token, SECRET)
                return [
                    {'key': 'DATABASE_URL', 'value': SECRET},
                    {'key': 'EXPECTED_CHAIN_ID', 'value': 'columbus-5'},
                    {'key': 'LCD_URLS', 'value': ','.join(f'https://provider-{index}.example/{SECRET}' for index in range(12))},
                    {'key': 'LCD_URLS', 'value': 'https://preview.example', 'is_preview': True},
                ]
            if url.endswith('/node_info'):
                return {'default_node_info': {'network': 'columbus-5'}, 'private_field': SECRET}
            return {'block': {'header': {'height': '123', 'time': '2026-10-09T07:00:00Z', 'private_field': SECRET}}}
        receipt = {}
        with patch.dict(os.environ, {'COOLIFY_URL': 'https://coolify.example', 'COOLIFY_WIRE_TOKEN': SECRET}), patch.object(PROBE, 'read_json', side_effect=read) as reads:
            PROBE.coolify_evidence(receipt, 'synthetic_app')
        self.assertEqual(receipt['configured_endpoint_count'], 12)
        self.assertEqual(receipt['unprobed_endpoint_count'], 4)
        self.assertEqual(len(receipt['provider_probes']), 8)
        self.assertEqual(reads.call_count, 17)
        self.assertNotIn(SECRET, json.dumps(receipt))
        self.assertNotIn('provider-0.example', json.dumps(receipt))

    def test_health_omits_unexpected_private_fields(self):
        payload = {'git_sha': 'a' * 40, 'private': SECRET, 'chain_identity': {
            'expected_chain_id': 'columbus-5', 'endpoints': [
                {'verified': True, 'observed_chain_id': 'columbus-5', 'url': SECRET, 'private': SECRET,
                 'provider': {'attempts': 12, 'circuit_open': True, 'private': SECRET, 'failures': SECRET}}]}}
        receipt = {}
        with patch.object(PROBE, 'read_json', return_value=payload):
            PROBE.health_evidence(receipt, 'https://public.example/health')
        self.assertEqual(receipt['health_status'], 'PASS')
        self.assertEqual(receipt['runtime_attestation'][0]['provider'], {'attempts': 12, 'circuit_open': True})
        self.assertNotIn(SECRET, json.dumps(receipt))

    def test_credentials_and_query_endpoint_fail_before_network(self):
        with patch.object(PROBE, 'read_json') as reads:
            for endpoint in ('https://user:password@example.com', f'https://example.com?token={SECRET}'):
                receipt = PROBE.provider_probe(0, endpoint)
                self.assertEqual(receipt['endpoint_validation']['status'], 'SKIP')
                self.assertNotIn(SECRET, json.dumps(receipt))
            reads.assert_not_called()

    def test_oversized_body_is_rejected_before_json_decode(self):
        class Response:
            def __enter__(self):
                return self
            def __exit__(self, *_args):
                return False
            def read(self, limit):
                self.limit = limit
                return b'x' * limit
        response = Response()
        with patch.object(PROBE.urllib.request, 'build_opener') as opener:
            opener.return_value.open.return_value = response
            with self.assertRaises(ValueError):
                PROBE.read_json('https://public.example')
        self.assertEqual(response.limit, PROBE.MAX_BYTES + 1)

    def test_hard_deadline_bounds_trickle_response_work(self):
        with patch.object(PROBE, 'REQUEST_TIMEOUT_SECONDS', 0.01):
            with self.assertRaises(PROBE.ProbeDeadline), PROBE.request_deadline():
                time.sleep(0.05)

    def test_redirect_is_not_followed(self):
        handler = PROBE.NoRedirect()
        self.assertIsNone(handler.redirect_request(None, None, 302, 'redirect', {}, 'https://elsewhere.example'))

    def test_pinned_pool_probe_redacts_body_and_rejects_missing_wrong_header(self):
        for header, outcome in ((None, 'MissingHeightHeader'), ('122', 'HeightMismatch'), ('123', None)):
            with self.subTest(header=header), patch.object(PROBE, 'read_json', return_value=({'data': SECRET}, header)) as read:
                receipt = PROBE.pinned_pool_probe('https://provider.example', 'terra1synthetic', '123')
            self.assertEqual(read.call_args.kwargs, {'height': '123', 'include_height': True})
            self.assertNotIn(SECRET, json.dumps(receipt))
            if outcome is None:
                self.assertEqual(receipt, {'status': 'PASS'})
            else:
                self.assertEqual(receipt, {'status': 'SKIP', 'error_type': outcome})

    def test_pinned_probe_without_latest_height_performs_no_read(self):
        with patch.object(PROBE, 'read_json') as read:
            receipt = PROBE.pinned_pool_probe('https://provider.example', 'terra1synthetic', None)
            read.assert_not_called()
        self.assertEqual(receipt['status'], 'SKIP')

    def test_malformed_height_is_not_a_success(self):
        with patch.object(PROBE, 'read_json', return_value={'block': {'header': {'height': SECRET}}}):
            receipt = PROBE.provider_probe(0, 'https://public.example')
        self.assertEqual(receipt['latest']['status'], 'SKIP')
        self.assertNotIn('latest_status', receipt)
        self.assertNotIn(SECRET, json.dumps(receipt))


if __name__ == '__main__':
    unittest.main()
