#!/usr/bin/env python3
"""No-production workflow, containment and deployed-pin acceptance regressions."""
import importlib.util
import os
import json
from pathlib import Path
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch

HERE = Path(__file__).parent
def load(name, file):
    spec = importlib.util.spec_from_file_location(name, HERE/file)
    m = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(m)
    return m
custody = load('custody_test', 'complete-custody.py')
containment = load('containment_test', 'route-containment.py')
deploy = load('deployment_test', 'deploy-route-pins.py')


class FakeChain:
    def __init__(self):
        self.pauses = {'a': False, 'b': True, 'c': True}
        self.receipts, self.actions = {}, []
    def network(self): return 'columbus-5'
    def info(self, addr): return {'code_id': 10, 'admin': 'gov'}
    def code_hash(self, code): return 'a'*64
    def smart(self, pair, msg): return {'paused': self.pauses[pair]}
    def receipt(self, hash_value): return self.receipts.get(hash_value)
    def validate_saved(self, action, entry):
        assert entry['action'] == action
    def transaction(self, action, signed):
        tx_hash = f'{len(self.actions)+1:064X}'
        signed(tx_hash, '/external/signed.json', 7)
        self.actions.append(action)
        self.pauses[action['msg']['set_pair_paused']['pair']] = action['msg']['set_pair_paused']['paused']
        result = {'code': 0, 'txhash': tx_hash}
        self.receipts[tx_hash] = result
        return result


class WorkflowTest(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.journal = {'status': 'complete', 'identity_hash': 'release', 'codes': {'factory': 10},
            'operations': {}, 'identity': {'config': {'chain_id': 'columbus-5', 'factory': 'factory',
                'governance': 'gov', 'max_total_fee_uluna': 100}, 'artifacts': {'factory': {'sha256': 'a'*64}}},
            'inventory': {'pairs': [{'address': 'a', 'paused': False}, {'address': 'b', 'paused': False},
                {'address': 'c', 'paused': True}], 'affected_pairs': ['a', 'b', 'c']}}
        self.write('journal.json', self.journal)
    def write(self, name, value):
        p = self.root/name
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_text(json.dumps(value))

    def test_one_prompt_adapter_validates_keys_and_restores_process_environment(self):
        self.journal['identity']['config']['transport'] = {
            'home': '/synthetic/home', 'keyring_backend': 'file',
            'node': 'https://synthetic.invalid', 'lcd': 'https://synthetic.invalid'}
        self.write('journal.json', self.journal)
        observations = []
        chain = SimpleNamespace(unlock=lambda: observations.append(
            os.environ.get('TERRAD_HOST_KEYRING_PASS')))
        with patch.object(custody, 'STATE', self.root), \
             patch.object(custody, 'TerradChain', return_value=chain), \
             patch.dict(os.environ, {'TERRAD_HOST_KEYRING_PASS': 'prior-synthetic-value'}):
            self.assertIs(chain, custody.adapter({'TERRAD_HOST_KEYRING_PASS': 'synthetic-pass'}))
            self.assertEqual(['synthetic-pass'], observations)
            self.assertEqual('prior-synthetic-value', os.environ['TERRAD_HOST_KEYRING_PASS'])

    def test_failed_key_identity_validation_clears_temporary_process_environment(self):
        self.journal['identity']['config']['transport'] = {
            'home': '/synthetic/home', 'keyring_backend': 'file',
            'node': 'https://synthetic.invalid', 'lcd': 'https://synthetic.invalid'}
        self.write('journal.json', self.journal)
        def reject():
            raise RuntimeError('Key identity mismatch')
        with patch.object(custody, 'STATE', self.root), \
             patch.object(custody, 'TerradChain', return_value=SimpleNamespace(unlock=reject)), \
             patch.dict(os.environ, {}, clear=True):
            with self.assertRaisesRegex(RuntimeError, 'Key identity mismatch'):
                custody.adapter({'TERRAD_HOST_KEYRING_PASS': 'synthetic-pass'})
            self.assertNotIn('TERRAD_HOST_KEYRING_PASS', os.environ)

    def test_completed_rollout_resume_verifies_without_execute_or_finalize(self):
        self.write('live-smoke/evidence.json', {'status': 'PASS'})
        self.write('application-rollout.json', {'status': 'finished', 'live_readback': {'ok': True}})
        calls = []
        smoke = SimpleNamespace(run_smoke=lambda *a: {'status': 'PASS'})
        deployment = SimpleNamespace(live_trading_readback=lambda: {'response': {'tax_kind': 'route_local_v1'}})
        with patch.object(custody, 'STATE', self.root), patch.object(custody, 'adapter', return_value=object()), \
             patch.object(custody, 'load_module', side_effect=lambda name, file: smoke if 'smoke' in file else deployment if 'deploy' in file else object()), \
             patch.object(custody, 'run', side_effect=lambda cmd, env=None: calls.append(cmd)), \
             patch.object(custody, 'provider_readback'):
            custody.complete_route({})
        commands = ' '.join(' '.join(c) for c in calls)
        self.assertNotIn('route-02-execute', commands)
        self.assertNotIn('route-04-finalize', commands)
        self.assertIn('route-03-verify', commands)

    def test_exact_containment_preserves_original_and_external_pauses(self):
        chain = FakeChain()
        containment.contain(chain, self.root, 'smoke failed')
        saved = json.loads((self.root/'containment.json').read_text())
        self.assertEqual(['a'], saved['introduced'])
        self.assertEqual(['b'], saved['external_pauses'])
        self.assertEqual(1, len(chain.actions))
        containment.contain(chain, self.root, 'resume')
        self.assertEqual(1, len(chain.actions))
        with self.assertRaisesRegex(RuntimeError, 'No smoke evidence'):
            containment.restore_after_success(chain, self.root)
        smoke_receipt = {'code': 0, 'txhash': 'F'*64}
        chain.receipts[smoke_receipt['txhash']] = smoke_receipt
        self.write('live-smoke/evidence.json', {'status': 'PASS', 'manifest_identity': 'release', 'receipts': [smoke_receipt]})
        containment.restore_after_success(chain, self.root)
        self.assertEqual({'a': False, 'b': True, 'c': True}, chain.pauses)

    def test_unknown_containment_inclusion_never_resigns(self):
        chain = FakeChain()
        containment.contain(chain, self.root, 'smoke failed')
        chain.receipts.clear()
        with self.assertRaisesRegex(RuntimeError, 'inclusion unknown'):
            containment.contain(chain, self.root, 'resume')
        self.assertEqual(1, len(chain.actions))

    def test_aggregate_core_smoke_and_containment_budget_checked_before_broadcast(self):
        chain = FakeChain()
        self.journal['operations'] = {'store': {'fee_uluna': 95}}
        self.write('journal.json', self.journal)
        with self.assertRaisesRegex(RuntimeError, 'aggregate fee cap'):
            containment.contain(chain, self.root, 'smoke failed')
        self.assertEqual([], chain.actions)

    def test_provider_raw_failure_is_saved_and_never_claimed_pass(self):
        response = SimpleNamespace(stdout='{"status":"FAIL","checks":[{"error":"out of gas"}]}', stderr='failure', returncode=1)
        with patch.object(custody, 'STATE', self.root), patch.object(custody.subprocess, 'run', return_value=response):
            with self.assertRaisesRegex(RuntimeError, 'raw JSON/errors saved'):
                custody.provider_readback({})
        self.assertIn('out of gas', (self.root/'provider-readback.json').read_text())
        self.assertEqual('failure', (self.root/'provider-readback.stderr').read_text())

    def test_live_public_readback_rejects_stale_runtime_revision(self):
        class Response:
            def __enter__(self): return self
            def __exit__(self, *args): pass
            def read(self, *args): return b'{"git_sha":"old"}'
        with patch.object(deploy.urllib.request, 'urlopen', return_value=Response()):
            with self.assertRaisesRegex(AssertionError, 'revision readback'):
                deploy.public_readback({})

    def test_new_catalog_pin_does_not_substitute_for_trading_capability(self):
        class Response:
            def __init__(self, value): self.value = value
            def __enter__(self): return self
            def __exit__(self, *args): pass
            def read(self, *args): return json.dumps(self.value).encode()
        replies = [Response({'tax_kind': 'sell_in', 'router_hops_tax': True, 'estimated_amount_out': '10'})]
        with patch.object(deploy.urllib.request, 'urlopen', side_effect=replies):
            with self.assertRaisesRegex(AssertionError, 'trading trust/capability'):
                deploy.live_trading_readback()

    def test_missing_public_env_uses_post_and_existing_pin_uses_patch(self):
        env = {'ROUTER_ADDRESS': 'old'}
        methods = []
        def api(path, method='GET', data=None):
            if method == 'GET':
                return [{'key': k, 'value': v, 'is_preview': False} for k, v in env.items()]
            methods.append((data['key'], method))
            if method == 'PATCH': self.assertIn(data['key'], env)
            if method == 'POST': self.assertNotIn(data['key'], env)
            env[data['key']] = data['value']
        with patch.object(deploy, 'api', side_effect=api):
            deploy.apply_public_pins(deploy.INDEXER, {'ROUTER_ADDRESS': 'new', 'EXPECTED_CHAIN_ID': 'columbus-5'})
        self.assertEqual([('ROUTER_ADDRESS', 'PATCH'), ('EXPECTED_CHAIN_ID', 'POST')], methods)

    def test_paused_configuration_readback_reads_dynamic_chunks_without_quote(self):
        class Response:
            def __init__(self, value): self.value = value
            def __enter__(self): return self
            def __exit__(self, *args): pass
            def read(self, *args): return self.value
        urls = []
        def urlopen(request, timeout):
            url = request.full_url
            urls.append(url)
            self.assertNotIn('/route/solve', url)
            if url.endswith('/health'): return Response(json.dumps({'git_sha': deploy.REVISION}).encode())
            if 'community-tokens' in url: return Response(b'{"configured":true,"code_id":50000}')
            if url.endswith('/'): return Response(b'<script type="module" src="/assets/index.js"></script>')
            if url.endswith('/assets/index.js'): return Response(b'import("./TokenPage.js"); const router="router-pin";')
            if url.endswith('/assets/TokenPage.js'): return Response(b'const tokenCode="50000";')
            raise AssertionError(url)
        with patch.object(deploy.urllib.request, 'urlopen', side_effect=urlopen):
            result = deploy.public_readback({'COMMUNITY_TAX_CODE_ID': '50000',
                'VITE_COMMUNITY_TAX_CODE_ID': '50000', 'VITE_ROUTER_ADDRESS': 'router-pin'})
        self.assertIn('https://dex.cl8y.com/assets/TokenPage.js', result['frontend_chunks'])

    def test_live_trading_occurs_only_after_finalization_and_actual_smoke(self):
        self.journal['status'] = 'contracts_ready_paused'
        self.write('journal.json', self.journal)
        self.write('application-rollout.json', {'status': 'finished', 'live_readback': {'ok': True}})
        order = []
        def run(cmd, env=None):
            order.append(Path(cmd[-1]).name)
            if 'route-04-finalize.sh' in cmd[-1]:
                self.journal['status'] = 'complete'
                self.write('journal.json', self.journal)
        def smoke_run(*args):
            self.assertEqual('complete', self.journal['status'])
            order.append('smoke')
            self.write('live-smoke/evidence.json', {'status': 'PASS'})
        def trading():
            self.assertIn('smoke', order)
            order.append('trading')
            return {'response': {'tax_kind': 'route_local_v1'}}
        smoke = SimpleNamespace(run_smoke=smoke_run)
        deployment = SimpleNamespace(live_trading_readback=trading)
        with patch.object(custody, 'STATE', self.root), patch.object(custody, 'adapter', return_value=object()), \
             patch.object(custody, 'load_module', side_effect=lambda name, file: smoke if 'smoke' in file else deployment if 'deploy' in file else object()), \
             patch.object(custody, 'run', side_effect=run), patch.object(custody, 'provider_readback'):
            custody.complete_route({})
        self.assertLess(order.index('route-04-finalize.sh'), order.index('smoke'))
        self.assertLess(order.index('smoke'), order.index('trading'))

    def test_new_containment_round_after_restoration_retains_prior_fee_receipts(self):
        chain = FakeChain()
        containment.contain(chain, self.root, 'first app failure')
        receipt = {'code': 0, 'txhash': 'F'*64}
        chain.receipts[receipt['txhash']] = receipt
        self.write('live-smoke/evidence.json', {'status': 'PASS', 'manifest_identity': 'release', 'receipts': [receipt]})
        containment.restore_after_success(chain, self.root)
        containment.contain(chain, self.root, 'second app failure')
        saved = json.loads((self.root/'containment.json').read_text())
        self.assertEqual(2, saved['round'])
        self.assertEqual('contained', saved['status'])
        self.assertEqual(3, len(saved['operations']))
        self.assertEqual(21, containment.aggregate_fees(self.root, saved)[0])
        self.assertTrue(chain.pauses['a'])


if __name__ == '__main__':
    unittest.main(verbosity=2)
