"""Exercise the complete bounded smoke, including rejected public data, without networking."""
import contextlib
from datetime import datetime, timedelta, timezone
import importlib.util
import io
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
from urllib.parse import parse_qs, urlsplit

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location('live_evidence', ROOT / 'scripts/qa/live-evidence-1206-1211.py')
PROBE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(PROBE)
REVISION = 'a' * 40

class Response:
    def __init__(self, code, body):
        self.code, self.body = code, json.dumps(body).encode()
    def read(self, limit):
        return self.body[:limit]
    def __enter__(self):
        return self
    def __exit__(self, *args):
        return False

class Opener:
    def __init__(self, invalid=None):
        self.calls, self.invalid = [], invalid
    def open(self, request, timeout):
        assert timeout == 4
        parts = urlsplit(request.full_url)
        self.calls.append((parts.path, request.get_method()))
        today = datetime.now(timezone.utc).date()
        if parts.path == '/health':
            return Response(200, {'git_sha': REVISION})
        if parts.path == '/api-docs/openapi.json':
            return Response(200, {'paths': {'/api/v1/evidence/digest': {}}})
        if parts.path == '/api/v1/protocol/fees':
            rows = [{'cohort': c, 'event_count': 0, 'amount_usd': '0', 'share_pct': None} for c in ('retail', 'mm', 'unknown')]
            if self.invalid == 'exponent':
                rows[0]['amount_usd'] = '1E-18'
            if self.invalid == 'bool_count':
                rows[0]['event_count'] = True
            return Response(200, {'by_cohort': rows})
        if parts.path == '/api/v1/evidence/digest':
            if request.get_method() == 'POST':
                return Response(405, {})
            query = parse_qs(parts.query)
            if len(query) != 1 or query.get('day', [''])[0] not in (str(today), str(today - timedelta(days=1))):
                return Response(400, {})
            counts = {'swap': {'swap': 1}, 'lp': {'add': 0, 'remove': 0}, 'limit': {'place': 0, 'cancel': 0, 'fill': 0}, 'wrap': {'wrap': 0, 'unwrap': 0}, 'window': {'ust1_mint': 0, 'ust1_redeem': 0}}
            value = {'day': query['day'][0], 'timezone': 'UTC', 'complete': query['day'][0] != str(today), 'counts': counts,
                'fees': {'total_usd': '0', 'by_source': [{'source': s, 'event_count': 0, 'amount_usd': '0', 'share_pct': None} for s in PROBE.SOURCES], 'by_token': []}}
            if self.invalid == 'fee_missing':
                del value['fees']['by_source'][0]['event_count']
            if self.invalid == 'actor':
                value['sender'] = 'synthetic_actor'
            return Response(200, value)
        raise AssertionError('unexpected network path')

class LiveEvidenceTests(unittest.TestCase):
    def run_probe(self, invalid=None):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / 'receipt.json'
            opener = Opener(invalid)
            with patch('sys.argv', ['probe', '--revision', REVISION, '--output', str(output)]), patch.object(PROBE.urllib.request, 'build_opener', return_value=opener), contextlib.redirect_stdout(io.StringIO()):
                code = PROBE.main()
            return code, json.loads(output.read_text()), opener
    def test_complete_smoke_uses_exact_budget_and_checks_both_revisions(self):
        code, receipt, opener = self.run_probe()
        self.assertEqual(code, 0)
        self.assertEqual(receipt['overall'], 'PASS')
        self.assertEqual(receipt['requests'], 14)
        self.assertEqual(len(opener.calls), 14)
        self.assertTrue(receipt['checks']['serving_revision_after'])
    def test_identity_contamination_fails_without_exporting_body(self):
        code, receipt, _ = self.run_probe('actor')
        self.assertEqual(code, 1)
        self.assertNotIn('synthetic_actor', json.dumps(receipt))
    def test_scientific_cohort_amount_fails(self):
        code, receipt, _ = self.run_probe('exponent')
        self.assertEqual(code, 1)
        self.assertEqual(receipt['failed_check'], 'fee_plain_decimal')

    def test_missing_digest_fee_fields_fail(self):
        code, receipt, _ = self.run_probe('fee_missing')
        self.assertEqual(code, 1)
        self.assertEqual(receipt['failed_check'], 'fee_row_shape')
    def test_boolean_cohort_count_fails(self):
        code, receipt, _ = self.run_probe('bool_count')
        self.assertEqual(code, 1)
        self.assertEqual(receipt['failed_check'], 'fee_count')

if __name__ == '__main__':
    unittest.main()
