#!/usr/bin/env python3
"""Owned bounded root-manifest transport tests; no live credentials or reads."""
import hashlib
import importlib.util
import unittest
from pathlib import Path

SPEC = importlib.util.spec_from_file_location("root_attester", Path(__file__).with_name("attest-trading-quotes-1473.py"))
ATTEST = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(ATTEST)
BOOTSTRAPS = ("/bootstrap/theme.js", "/bootstrap/trade-shell.js")
INDEX = "/assets/index-Ab_12.js"


def document(paths=(*BOOTSTRAPS, INDEX)):
    return ("<!doctype html><html><head>" + "".join(f'<script src="{path}"></script>' for path in paths)
            + "</head><body></body></html>").encode()


class Reader:
    def __init__(self, html, payloads=None):
        self.html = html
        self.payloads = payloads or {path: ("owned public " + path).encode() for path in (*BOOTSTRAPS, INDEX)}
        self.calls = []

    def read(self, url, limit, private=False):
        self.calls.append((url, limit, private))
        if len(self.calls) > ATTEST.MAX_READS:
            raise ValueError("read_budget")
        if url == ATTEST.FRONTEND + "/":
            value = self.html
        else:
            assert url.startswith(ATTEST.FRONTEND + "/")
            value = self.payloads[url.removeprefix(ATTEST.FRONTEND)]
        if len(value) > limit:
            raise ValueError("body_bound")
        return value


class RootArtifacts(unittest.TestCase):
    def assert_manifest_rejected_before_asset_reads(self, html):
        reader = Reader(html)
        with self.assertRaises(ValueError) as caught:
            ATTEST.artifacts(reader)
        self.assertEqual(len(reader.calls), 1)
        self.assertNotIn("credential", str(caught.exception))
        self.assertNotIn("https://", str(caught.exception))

    def test_all_three_live_root_scripts_are_hashed_and_bind_public_paths(self):
        html = document()
        reader = Reader(html)
        result = ATTEST.artifacts(reader)
        self.assertEqual(result["html_sha256"], hashlib.sha256(html).hexdigest())
        self.assertEqual([row["path"] for row in result["root_scripts"]], [*BOOTSTRAPS, INDEX])
        for row in result["root_scripts"]:
            self.assertEqual(set(row), {"path", "asset", "sha256", "bytes"})
            self.assertEqual(row["asset"], row["path"].rsplit("/", 1)[-1])
            self.assertEqual(row["sha256"], hashlib.sha256(reader.payloads[row["path"]]).hexdigest())
            self.assertEqual(row["bytes"], len(reader.payloads[row["path"]]))
        self.assertEqual([call[1] for call in reader.calls], [128 * 1024] + [8 * 1024 * 1024] * 3)
        self.assertTrue(all(call[2] is False for call in reader.calls))

    def test_markup_order_is_preserved_and_html_parser_handles_unquoted_uppercase(self):
        html = (f'<SCRIPT SRC={INDEX}></SCRIPT><script defer src="{BOOTSTRAPS[1]}"></script>'
                f"<script src='{BOOTSTRAPS[0]}'></script>").encode()
        result = ATTEST.artifacts(Reader(html))
        self.assertEqual([row["path"] for row in result["root_scripts"]], [INDEX, BOOTSTRAPS[1], BOOTSTRAPS[0]])

    def test_comment_and_inline_script_text_do_not_manufacture_external_artifacts(self):
        html = (b'<!-- <script src="https://foreign.invalid/credential.js"></script> -->'
                b'<script>const sample = "<script src=/ignored.js>";</script>' + document())
        result = ATTEST.artifacts(Reader(html))
        self.assertEqual(len(result["root_scripts"]), 3)

    def test_unknown_foreign_and_noncanonical_scripts_fail_before_any_asset_reads(self):
        for path in ("https://foreign.invalid/credential.js", "https://dex.cl8y.com/assets/index.js",
                     "//foreign.invalid/credential.js", "/unexpected.js", "/bootstrap/unknown.js",
                     "assets/index.js", "/assets/../credential.js", "/assets/a..js", "/assets/a.js?credential=secret",
                     "/assets/a.js#credential", "/%61ssets/a.js", "/assets/a%2e.js", "/assets/a\\b.js",
                     "/assets/ü.js", "/assets/" + "a" * 150 + ".js", ""):
            with self.subTest(path=path):
                self.assert_manifest_rejected_before_asset_reads(document((*BOOTSTRAPS, INDEX, path)))

    def test_duplicate_paths_and_cross_directory_duplicate_basenames_are_rejected(self):
        for paths in ((*BOOTSTRAPS, INDEX, INDEX), (*BOOTSTRAPS, INDEX, BOOTSTRAPS[0]),
                      (*BOOTSTRAPS, INDEX, "/assets/theme.js"), (*BOOTSTRAPS, INDEX, "/assets/trade-shell.js")):
            with self.subTest(paths=paths):
                self.assert_manifest_rejected_before_asset_reads(document(paths))

    def test_root_requires_both_bootstraps_and_an_application_asset(self):
        for paths in ((), (INDEX,), (BOOTSTRAPS[0], INDEX), (BOOTSTRAPS[1], INDEX), BOOTSTRAPS):
            with self.subTest(paths=paths):
                self.assert_manifest_rejected_before_asset_reads(document(paths))

    def test_eighth_external_script_allowed_ninth_rejected_without_fetching_assets(self):
        paths = (*BOOTSTRAPS, *(f"/assets/chunk-{i}.js" for i in range(6)))
        payloads = {path: b"bounded owned script" for path in paths}
        reader = Reader(document(paths), payloads)
        self.assertEqual(len(ATTEST.artifacts(reader)["root_scripts"]), 8)
        self.assertEqual(len(reader.calls), 9)
        self.assert_manifest_rejected_before_asset_reads(document((*paths, "/assets/ninth.js")))

    def test_base_and_ambiguous_duplicate_src_attributes_fail_closed(self):
        for html in (b'<base href="https://foreign.invalid/credential/">' + document(),
                     b'<base href="/">' + document(),
                     document().replace(b'src="/bootstrap/theme.js"', b'src="/bootstrap/theme.js" src="/assets/a.js"')):
            with self.subTest(html=html):
                self.assert_manifest_rejected_before_asset_reads(html)

    def test_empty_and_oversize_script_bodies_cannot_establish_an_artifact(self):
        for payload in (b"", b"x" * (8 * 1024 * 1024 + 1)):
            payloads = {path: b"known public bytes" for path in (*BOOTSTRAPS, INDEX)}
            payloads[INDEX] = payload
            with self.subTest(size=len(payload)), self.assertRaises(ValueError):
                ATTEST.artifacts(Reader(document(), payloads))


if __name__ == "__main__":
    unittest.main()
