#!/usr/bin/env python3
"""Check #1212 source policy or nginx HTTP responses without a wallet/chain."""

import argparse
import re
import sys
import urllib.error
import urllib.parse
import urllib.request
import xml.etree.ElementTree as ET
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
ROBOTS = "User-agent: *\nDisallow:\nSitemap: https://dex.cl8y.com/sitemap.xml\n"
SITEMAP_ROOT = "{http://www.sitemaps.org/schemas/sitemap/0.9}urlset"
CRAWL_FILES = {"robots.txt": "text/plain", "sitemap.xml": "application/xml"}
SIBLINGS = (
    "/sitemap_index.xml", "/robots.txt.bak", "/robots.txt/",
    "/sitemap.xml/", "/sitemap.xml.bak", "/robots.txt.bak.js",
)


def require(condition, message):
    if not condition:
        raise ValueError(message)


def check_body(name, body):
    require(b"<!doctype" not in body.lower(), f"{name}: received SPA HTML")
    if name == "robots.txt":
        require(body.decode("utf-8") == ROBOTS,
                "robots.txt: expected only the fixed crawler directives and DEX sitemap")
    else:
        require(b"<!--" not in body, "sitemap.xml: keep the empty sitemap free of comments/claims")
        root = ET.fromstring(body)
        require(root.tag == SITEMAP_ROOT, "sitemap.xml: expected namespaced urlset")
        require(not list(root) and not root.attrib and not (root.text or "").strip(),
                "sitemap.xml: keep the transaction SPA sitemap empty")


def check_source():
    for name in CRAWL_FILES:
        check_body(name, (REPO_ROOT / "frontend-dapp/public" / name).read_bytes())
    config = (REPO_ROOT / "docker/frontend/nginx.conf").read_text()
    # Strip the nested empty types block before inspecting locations.
    flat = re.sub(r"types\s*\{\s*\}", "types-cleared;", config)
    for name, media_type in CRAWL_FILES.items():
        match = re.search(r"location\s+=\s+/" + re.escape(name) + r"\s*\{([^}]+)\}", flat)
        require(match is not None, f"nginx: missing exact /{name} location")
        block = match.group(1)
        for directive in ("types-cleared;", f"default_type {media_type};", "try_files $uri =404;"):
            require(directive in block, f"nginx /{name}: missing {directive}")
    shell = re.search(r"location\s+=\s+/index\.html\s*\{([^}]+)\}", flat)
    require(shell is not None and 'add_header X-Robots-Tag "noindex, nofollow" always;' in shell.group(1),
            "nginx: expected noindex/nofollow on the shared index.html shell")
    sibling = re.search(r"location\s+~\*\s+\^/\(robots[^\n]+\{([^}]+)\}", flat)
    require(sibling is not None and "sitemap" in sibling.group(0) and "return 404;" in sibling.group(1),
            "nginx: expected crawl-file sibling 404 protection")
    print("PASS source: fixed robots, empty XML sitemap, file-only/sibling 404s, shell noindex")


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        return None


def fetch(base_url, path):
    opener = urllib.request.build_opener(NoRedirect())
    request = urllib.request.Request(base_url + path, headers={"User-Agent": "cl8y-qa-1212"})
    try:
        response = opener.open(request, timeout=8)
    except urllib.error.HTTPError as error:
        response = error
    with response:
        return response.status, response.headers, response.read()


def check_http(base_url, missing_files):
    parsed = urllib.parse.urlsplit(base_url)
    require(parsed.scheme in ("http", "https") and parsed.netloc
            and parsed.path in ("", "/") and not parsed.query and not parsed.fragment
            and parsed.username is None and parsed.password is None,
            "--base-url must be an HTTP(S) origin without credentials, query, or path")
    base_url = base_url.rstrip("/")
    for name, media_type in CRAWL_FILES.items():
        status, headers, body = fetch(base_url, "/" + name)
        if missing_files:
            require(status == 404, f"missing {name}: expected 404, got {status}")
        else:
            require(status == 200, f"{name}: expected 200, got {status}")
            actual_type = headers.get_content_type()
            require(actual_type == media_type, f"{name}: expected {media_type}, got {actual_type}")
            check_body(name, body)
            require("max-age=3600" in headers.get("Cache-Control", ""), f"{name}: missing bounded cache")
            require(headers.get("X-Content-Type-Options") == "nosniff", f"{name}: missing nosniff")
        print(f"PASS /{name}: {status} {headers.get_content_type()}")

    for path in SIBLINGS:
        status, _, _ = fetch(base_url, path)
        require(status == 404, f"{path}: expected 404, got {status}")
    print("PASS crawl-file siblings: all 404")

    status, _, shell = fetch(base_url, "/index.html")
    require(status == 200 and b"<!doctype html" in shell.lower(), "index.html: expected SPA shell")
    for path in ("/", "/trade", "/portfolio", "/trader/qa-1212", "/token/qa-1212/manage", "/mint"):
        status, headers, body = fetch(base_url, path)
        require(status == 200 and body == shell, f"{path}: expected unchanged SPA shell")
        require(headers.get("X-Robots-Tag") == "noindex, nofollow", f"{path}: missing noindex header")
        cache = headers.get("Cache-Control", "")
        require("no-cache" in cache and "immutable" not in cache, f"{path}: incorrect shell caching")
    print("PASS SPA routes: same HTML, noindex/nofollow, no-cache")

    status, headers, body = fetch(base_url, "/og-image.png")
    require(status == 200 and headers.get_content_type() == "image/png"
            and body.startswith(b"\x89PNG\r\n\x1a\n"), "og-image.png: expected real PNG")
    for extension in ("js", "css"):
        status, headers, _ = fetch(base_url, f"/assets/qa-1212-missing.{extension}")
        require(status == 404, f"missing {extension}: expected 404, got {status}")
        cache = headers.get("Cache-Control", "")
        require("no-store" in cache and "immutable" not in cache, f"missing {extension}: incorrect cache")
        # Check a shipped asset referenced by the shell, not an assumed build hash.
        paths = re.findall(rb'["\'](/assets/[^"\']+\.' + extension.encode() + rb')["\']', shell)
        require(paths, f"index.html: no {extension} asset available to verify")
        status, headers, _ = fetch(base_url, paths[0].decode())
        require(status == 200 and "immutable" in headers.get("Cache-Control", ""),
                f"present {extension}: expected immutable 200")
    print("PASS adjacent assets: real OG PNG, immutable JS/CSS, no-store 404 misses")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base-url", help="Check HTTP instead of repository source")
    parser.add_argument("--missing-files", action="store_true", help="Expect both crawl files to be absent")
    args = parser.parse_args()
    try:
        if args.base_url:
            check_http(args.base_url, args.missing_files)
        else:
            require(not args.missing_files, "--missing-files requires --base-url")
            check_source()
    except (OSError, ValueError, ET.ParseError) as error:
        print(f"FAIL #1212: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
