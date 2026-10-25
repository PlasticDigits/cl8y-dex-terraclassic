"""Synthetic clock/HTTP controls for the durable monitoring-only alert drill."""
import copy
import json
import tempfile
import unittest
import subprocess
import sys
from pathlib import Path

import drill


class LoopSignalTests(unittest.TestCase):
    def execute(self, body):
        program = ("import importlib.util, os, signal, threading, time\n"
                   f"s=importlib.util.spec_from_file_location('drill_fixture',{str(Path(drill.__file__).resolve())!r})\n"
                   "m=importlib.util.module_from_spec(s);s.loader.exec_module(m)\n" + body)
        return subprocess.run([sys.executable, "-I", "-c", program], capture_output=True,
                              check=True, timeout=10).stdout.decode()

    def test_term_and_int_handlers_return_while_event_condition_lock_is_held(self):
        for number in (15, 2):
            with self.subTest(signal=number):
                output = self.execute(
                    "event=threading.Event(); steps=[]\n"
                    "previous=signal.getsignal(signal.SIGTERM)\n"
                    "m.run_once=lambda:steps.append('step')\n"
                    "def idle(seconds):\n"
                    " assert seconds==5\n"
                    " with event._cond:\n"
                    f"  os.kill(os.getpid(),{number})\n"
                    "  print('handler_returned',flush=True)\n"
                    "m.time.sleep=idle\n"
                    "assert m.run_loop()==0 and steps==['step']\n"
                    "assert signal.getsignal(signal.SIGTERM)==previous\n")
                self.assertEqual(output.strip(), "handler_returned")

    def test_signal_finishes_inflight_transition_and_does_not_start_another(self):
        output = self.execute(
            "steps=[]\n"
            "def finite_step():\n"
            " steps.append('started');os.kill(os.getpid(),signal.SIGTERM)\n"
            " time.sleep(0.02);steps.append('completed')\n"
            "m.run_once=finite_step\n"
            "assert m.run_loop()==0 and steps==['started','completed']\n"
            "print('finite_step_completed',flush=True)\n")
        self.assertEqual(output.strip(), "finite_step_completed")


class DrillTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.state = str(Path(self.directory.name) / "state.json")
        self.receipt = str(Path(self.directory.name) / "receipt.json")
        self.now = 1000
        self.calls = []
        self.active = None
        self.deliveries = []
        self.error = False

    def read(self, endpoint, payload=None):
        self.calls.append((endpoint, payload))
        if self.error:
            raise TimeoutError("secret-private-endpoint")
        if endpoint == "toggle":
            return payload
        if endpoint == "active":
            return {"status": "success", "data": {"alerts": [] if self.active is None else [
                {"state": self.active, "labels": {**drill.IDENTITY, "private": "actor"},
                 "annotations": {"credential": "secret"}}]}}
        if endpoint == "receipts":
            return {"alerts": self.deliveries}
        raise AssertionError("unexpected_endpoint")

    def step(self):
        self.calls.clear()
        value = drill.step(self.state, self.receipt, read=self.read, clock=lambda: self.now)
        self.assertLessEqual(len(self.calls), 2)
        self.assertNotIn("secret", json.dumps(value))
        self.assertNotIn("private", Path(self.state).read_text())
        self.now += 5
        return value

    def delivery(self, status, timestamp=None, labels=None):
        return {"status": status, "timestamp": self.now if timestamp is None else timestamp,
                "labels": drill.IDENTITY if labels is None else labels}

    def test_pending_firing_delivery_and_recovery_are_required_and_persist(self):
        self.assertEqual(self.step()["phase"], "pending")
        self.assertEqual(self.calls, [("toggle", {"enabled": True})])
        self.active = "pending"
        self.assertEqual(self.step()["phase"], "firing")
        self.active = "firing"
        self.assertEqual(self.step()["phase"], "delivery")
        self.assertEqual(self.step()["phase"], "delivery", "firing without delivery cannot complete")
        self.deliveries = [self.delivery("firing")]
        self.assertEqual(self.step()["phase"], "recovery")
        self.assertEqual(self.calls[-1], ("toggle", {"enabled": False}))
        self.active = None
        self.assertEqual(self.step()["phase"], "recovery", "inactive without resolved delivery cannot complete")
        self.deliveries.append(self.delivery("resolved"))
        result = self.step()
        self.assertEqual(result["overall"], "PASS")
        self.assertEqual(set(result["markers_utc"]), drill.MARKERS)
        self.assertEqual(json.loads(Path(self.receipt).read_text())["overall"], "PASS")
        self.assertEqual(Path(self.receipt).stat().st_mode & 0o777, 0o600)
        self.assertEqual(self.step()["overall"], "PASS")
        self.assertEqual(self.calls, [], "replacement must retain proof and never repeat a finished drill")

    def test_missed_pending_is_an_executed_failure_even_if_delivery_recovers(self):
        self.step()
        self.active = "firing"
        self.step()
        self.deliveries = [self.delivery("firing")]
        self.step()
        self.active = None
        self.deliveries.append(self.delivery("resolved"))
        result = self.step()
        self.assertEqual(result["overall"], "FAIL")
        self.assertEqual(result["reason"], "pending_not_observed")
        self.assertEqual(self.step()["overall"], "FAIL")
        self.assertEqual(self.calls, [])

    def test_ambiguous_enable_is_not_renewed_and_timeout_fails_with_disable_attempt(self):
        self.error = True
        result = self.step()
        self.assertEqual(result["transport_errors"], 1)
        self.error = False
        self.assertEqual(self.step()["phase"], "pending")
        self.assertEqual(self.calls, [("active", None)])
        self.now = 1240
        result = self.step()
        self.assertEqual(result["overall"], "FAIL")
        self.assertEqual(result["reason"], "deadline")
        self.assertEqual(self.calls, [("toggle", {"enabled": False})])
        self.assertEqual(self.step()["overall"], "FAIL")
        self.assertEqual(self.calls, [])

    def test_old_or_wrong_service_notifications_cannot_prove_delivery(self):
        self.step()
        self.active = "pending"
        self.step()
        self.active = "firing"
        self.step()
        self.deliveries = [self.delivery("firing", timestamp=999),
                           self.delivery("firing", labels={**drill.IDENTITY, "job": "unrelated"})]
        self.assertEqual(self.step()["phase"], "delivery")
        self.assertEqual(self.calls, [("receipts", None)])

    def test_unbounded_remote_responses_and_duplicate_identity_fail_closed(self):
        for payload in ({}, {"status": "success", "data": {"alerts": [{}] * 65}}):
            with self.assertRaises(ValueError):
                drill.active_state(payload)
        alert = {"state": "pending", "labels": drill.IDENTITY}
        with self.assertRaisesRegex(ValueError, "alerts_duplicate"):
            drill.active_state({"status": "success", "data": {"alerts": [alert, copy.deepcopy(alert)]}})
        self.assertIsNone(drill.active_state({"status": "success", "data": {"alerts": [
            {**alert, "labels": {**drill.IDENTITY, "service": "wrong-service"}}]}}))
        with self.assertRaisesRegex(ValueError, "receipts_bound"):
            drill.delivered({"alerts": [{}] * 33}, "firing", 1000)

    def test_invalid_persisted_state_cannot_restart_or_publish_arbitrary_fields(self):
        Path(self.state).write_text('{"private":"secret"}')
        with self.assertRaisesRegex(ValueError, "state_shape"):
            self.step()
        self.assertEqual(self.calls, [])

    def test_transport_uses_fixed_internal_endpoint_timeout_and_body_bound(self):
        class Response:
            def __enter__(self):
                return self
            def __exit__(self, *_args):
                return None
            def read(self, limit):
                return b"x" * limit
        class Opener:
            def open(self, request, timeout):
                self.request, self.timeout = request, timeout
                return Response()
        original, fake = drill.OPENER, Opener()
        drill.OPENER = fake
        try:
            with self.assertRaisesRegex(ValueError, "body_bound"):
                drill.request("active")
            self.assertEqual(fake.timeout, 3)
            self.assertEqual(fake.request.full_url, "http://dex-prometheus:9090/api/v1/alerts")
            self.assertIsNone(drill.NoRedirect().redirect_request(None, None, 302, "", {}, "https://private/"))
        finally:
            drill.OPENER = original


if __name__ == "__main__":
    unittest.main()
