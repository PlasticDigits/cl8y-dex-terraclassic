"""Synthetic identity and output controls; never contacts Coolify."""
import importlib.util
import json
from pathlib import Path
import unittest

SPEC = importlib.util.spec_from_file_location('release', Path(__file__).with_name('trading-ui-release.py'))
R = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(R)


def app():
    return {'uuid': R.APP, 'id': 77, 'git_branch': 'main',
            'git_repository': 'https://git.cl8y.com/code/cl8y-dex-terraclassic',
            'build_pack': 'dockerfile', 'dockerfile_location': '/docker/frontend/Dockerfile',
            'fqdn': 'https://dex.cl8y.com', 'git_commit_sha': 'a' * 40}


class API:
    def __init__(self):
        self.app = app()
        self.detail = {'deployment_uuid': 'b' * 24, 'application_id': 77,
                       'commit': 'a' * 40, 'status': 'finished',
                       'logs': json.dumps([{'output': 'SECRET_PAYLOAD'},
                                           {'output': 'ERROR SECRET_PAYLOAD'},
                                           {'output': 'Error: No such container: cleanup'}])}
        self.calls = []

    def api(self, path):
        self.calls.append(path)
        return self.app if path.startswith('applications/') else self.detail


class ReleaseTests(unittest.TestCase):
    def test_binds_frontend_repository_and_image_recipe(self):
        for key, bad in [('uuid', 'indexer'), ('git_branch', 'other'),
                         ('git_repository', 'https://attacker.invalid/code/cl8y-dex-terraclassic'),
                         ('dockerfile_location', '/docker/indexer/Dockerfile'),
                         ('fqdn', 'https://indexer.dex.cl8y.com'), ('git_commit_sha', 'HEAD')]:
            api = API()
            api.app[key] = bad
            with self.subTest(key=key), self.assertRaises(ValueError):
                R.owned(api)

    def test_logs_yield_counts_without_downstream_secrets(self):
        api = API()
        result = R.deployment(api, 'b' * 24, 'a' * 40)
        self.assertEqual(result['overall'], 'FAIL')
        self.assertEqual(result['error_pattern_entries'], 1)
        self.assertEqual(result['missing_cleanup_container_entries'], 1)
        self.assertEqual(result['log_entries_inspected'], 3)
        self.assertNotIn('SECRET_PAYLOAD', json.dumps(result))
        self.assertEqual(api.calls, ['deployments/' + 'b' * 24, 'applications/' + R.APP])

    def test_pending_and_final_states_are_distinct_from_verified_success(self):
        for state in R.STATES:
            api = API()
            api.detail['status'] = state
            api.detail['logs'] = json.dumps([{'output': 'Build complete'},
                                              {'output': 'Error: No such container: cleanup'}])
            result = R.deployment(api, 'b' * 24, 'a' * 40)
            expected = ('PENDING' if state in {'queued', 'in_progress'} else
                        'PASS' if state == 'finished' else 'FAIL')
            with self.subTest(state=state):
                self.assertEqual(result['overall'], expected)
                self.assertEqual(result['error_pattern_entries'], 0)
                self.assertEqual(result['missing_cleanup_container_entries'], 1)

    def test_cleanup_phrase_does_not_hide_another_failure(self):
        api = API()
        api.detail['logs'] = json.dumps([{'output': 'Error: No such container: cleanup\nFATAL build failed'}])
        result = R.deployment(api, 'b' * 24, 'a' * 40)
        self.assertEqual(result['overall'], 'FAIL')
        self.assertEqual(result['error_pattern_entries'], 1)
        self.assertEqual(result['missing_cleanup_container_entries'], 0)

    def test_requires_exact_requested_revision_and_unchanged_pin(self):
        for revision in [None, '', 'HEAD', 'c' * 40]:
            with self.subTest(revision=revision), self.assertRaises(ValueError):
                R.deployment(API(), 'b' * 24, revision)
        api = API()
        api.app['git_commit_sha'] = 'c' * 40
        with self.assertRaises(ValueError):
            R.deployment(api, 'b' * 24, 'a' * 40)

    def test_rejects_other_deployment_and_malformed_log(self):
        for key, bad in [('deployment_uuid', 'c' * 24), ('application_id', 78),
                         ('status', 'untrusted'), ('commit', 'HEAD'), ('commit', 'c' * 40), ('logs', '"SECRET_PAYLOAD"')]:
            api = API()
            api.detail[key] = bad
            with self.subTest(key=key), self.assertRaises(ValueError):
                R.deployment(api, 'b' * 24, 'a' * 40)


if __name__ == '__main__':
    unittest.main()
