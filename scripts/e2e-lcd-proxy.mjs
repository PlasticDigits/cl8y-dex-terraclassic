#!/usr/bin/env node
/**
 * Loopback LCD proxy for Playwright E2E when host :1317 hangs (docker userland-proxy).
 * Forwards HTTP to `docker exec … curl http://127.0.0.1:1317…` (GitLab #292 / LT9).
 */
import http from 'node:http'
import { execFileSync } from 'node:child_process'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const REPO_ROOT = path.join(path.dirname(fileURLToPath(import.meta.url)), '..')
const COMPOSE = path.join(REPO_ROOT, 'docker-compose.yml')
const PORT = Number(process.env.E2E_LCD_PROXY_PORT || 13170)
const MAX_BODY = 32 * 1024 * 1024

function containerId() {
  return execFileSync('docker', ['compose', '-f', COMPOSE, 'ps', '-q', 'localterra'], {
    encoding: 'utf8',
    cwd: REPO_ROOT,
  }).trim()
}

function forward(method, urlPath) {
  const cid = containerId()
  if (!cid) throw new Error('localterra container not running')
  const inner = `http://127.0.0.1:1317${urlPath}`
  const out = execFileSync(
    'docker',
    ['exec', cid, 'curl', '-sS', '-X', method, '--max-time', '60', '-w', '\n__HTTP_CODE__%{http_code}', inner],
    { encoding: 'utf8', maxBuffer: MAX_BODY, cwd: REPO_ROOT }
  )
  const marker = out.lastIndexOf('\n__HTTP_CODE__')
  if (marker < 0) throw new Error('lcd proxy: bad curl response')
  const body = out.slice(0, marker)
  const status = Number(out.slice(marker + '\n__HTTP_CODE__'.length))
  return { status: Number.isFinite(status) ? status : 502, body }
}

const server = http.createServer((req, res) => {
  try {
    const { status, body } = forward(req.method || 'GET', req.url || '/')
    res.writeHead(status, { 'content-type': 'application/json' })
    res.end(body)
  } catch (e) {
    res.writeHead(502, { 'content-type': 'text/plain' })
    res.end(e instanceof Error ? e.message : String(e))
  }
})

server.listen(PORT, '127.0.0.1', () => {
  process.stdout.write(`e2e-lcd-proxy: http://127.0.0.1:${PORT} -> localterra:1317\n`)
})
