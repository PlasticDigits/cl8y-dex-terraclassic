import datetime
import importlib.util
from pathlib import Path
import unittest

SPEC = importlib.util.spec_from_file_location("stage_s3", Path(__file__).with_name("stage-logs-s3-credentials.py"))
STAGE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(STAGE)


class StagingTests(unittest.TestCase):
    def test_signed_read_is_deterministic_bounded_and_does_not_expose_secret(self):
        args = ("nbg1.your-objectstorage.com", "synthetic-logs", "nbg1", "synthetic-access-key", "synthetic-secret-key")
        now = datetime.datetime(2026, 10, 18, tzinfo=datetime.timezone.utc)
        request = STAGE.signing_request(*args, now=now)
        self.assertEqual(request.method, "HEAD")
        self.assertEqual(request.full_url, "https://nbg1.your-objectstorage.com/synthetic-logs")
        self.assertEqual(request.headers, STAGE.signing_request(*args, now=now).headers)
        self.assertNotIn("synthetic-secret-key", str(request.headers))
        self.assertIn("20261018/nbg1/s3/aws4_request", request.headers["Authorization"])
        self.assertIsNone(STAGE.NoRedirect().redirect_request(None, None, 302, "", {}, "https://private/"))

    def test_rejects_endpoint_credential_and_bucket_injection(self):
        args = ["https://nbg1.your-objectstorage.com", "synthetic-logs", "nbg1", "synthetic-access-key", "synthetic-secret-key"]
        for index, value in ((0, "http://private/"), (0, "https://user:pass@nbg1.your-objectstorage.com"),
                             (0, "https://nbg1.your-objectstorage.com/path"), (1, "private?secret"),
                             (2, "unbounded"), (3, "space secret key")):
            modified = list(args)
            modified[index] = value
            with self.assertRaises(ValueError):
                STAGE.signing_request(*modified)


if __name__ == "__main__":
    unittest.main()
