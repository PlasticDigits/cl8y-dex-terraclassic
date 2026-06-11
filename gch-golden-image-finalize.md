# Golden image finalize (cl8y-dex-terraclassic)

You are finishing configuration of a **GCH agent VM golden image** for the Terra Classic DEX project. The base OS packages, Docker, Rust, Node, Playwright, glab, and Cursor CLI are already installed.

Complete the remaining setup and verify everything works. Use passwordless sudo as needed (`sudo` without a password).

## Tasks

1. **Keplr wallet extension**
   - Download the official Keplr browser extension (unpacked) into `/home/agent/.gch/extensions/keplr`
   - Use a stable release appropriate for headless Chromium/Playwright automation

2. **Local Terra / Terrad**
   - Read this repo's README and docs for LocalTerra or Terrad setup
   - Install and configure per project conventions (e.g. docker-compose localterra if applicable)
   - Ensure the chain/node can start for local development

3. **Browser profile**
   - Ensure `/home/agent/.gch/browser-profile` exists and is usable for Playwright/Chromium with the Keplr extension

4. **Verify toolchain** — run and record results:
   - `rustc --version` and `cargo --version`
   - `docker ps` and `docker compose version`
   - `node --version` and Playwright Chromium launch smoke test
   - `agent about`
   - `glab --version`
   - Project build/test commands from this repo (e.g. `cargo test`, frontend tests if documented)

5. **Write report**
   - Summarize what you installed, configured, and verified
   - List any failures or manual follow-ups for the admin
   - Save to `/home/agent/.gch/golden-image-verify.log`

## Constraints

- Do **not** run pre-snapshot cleanup (admin runs that before imaging)
- Do **not** commit or push changes unless required to verify the build; if you commit, do not add Cursor attribution trailers
- Prefer project-documented versions and paths
