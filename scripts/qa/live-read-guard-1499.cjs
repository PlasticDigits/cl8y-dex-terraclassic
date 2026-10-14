// Pre-app read-only guard. Known public GET transport never enters CDP Fetch interception.
const IMAGE_PREFIX =
  "https://git.cl8y.com/code/cl8y-dex-terraclassic/raw/branch/main/tokenlist/images/";
const IMAGE_FILES = [
  "LUNC.png",
  "USTC.png",
  "CL8Y.png",
  "USTR.png",
  "ALPHA.png",
  "USTRIX.png",
  "SPACEUSD.png",
  "CLUNC.png",
  "CUSTC.png",
  "UST1.png",
  "VFDUSD.png",
  "USDT.png",
  "DO.png",
  "KENA.jpg",
];
function nativeBlockRules(frontend, backend) {
  const provider = ["cosmwasm", "cosmos", "terra", "rpc"];
  return {
    urlPatterns: [
      ...provider.flatMap((p) =>
        ["http", "https"].map((s) => ({
          urlPattern: `${s}://*:*/${p}/*`,
          block: true,
        })),
      ),
      { urlPattern: frontend + "/*", block: false },
      { urlPattern: backend + "/*", block: false },
      ...IMAGE_FILES.map((file) => ({
        urlPattern: IMAGE_PREFIX + file,
        block: false,
      })),
    ],
    urls: ["*"],
  };
}
function installReadGuard({ frontend, backend, remaining }) {
  if (!Number.isInteger(remaining) || remaining < 0 || remaining > 600)
    throw Error("invalid_guard_budget");
  const state = (window.__liveReadGuard1499 = {
    remaining,
    admitted: 0,
    rejected: 0,
    writes: 0,
    providers: 0,
    actors: 0,
    unexpected: 0,
    workers: 0,
  });
  const permit = (input, method, reserve) => {
    const url = new URL(
      typeof input === "string" ? input : input?.url || String(input),
      location.href,
    );
    if (!["GET", "HEAD", "OPTIONS"].includes(method)) {
      state.writes++;
      return false;
    }
    if (/\/(?:cosmwasm|cosmos|terra|rpc)(?:\/|$)/.test(url.pathname)) {
      state.providers++;
      return false;
    }
    if (url.searchParams.has("trader") || url.searchParams.has("sender")) {
      state.actors++;
      return false;
    }
    if (![frontend, backend].includes(url.origin)) {
      state.unexpected++;
      return false;
    }
    if (reserve && url.pathname.startsWith("/api/v1/route/solve")) {
      if (!state.remaining) {
        state.rejected++;
        return false;
      }
      state.remaining--;
      state.admitted++;
    }
    return true;
  };
  const refused = () =>
    new DOMException("Read-only probe blocked request", "SecurityError");
  const fetch = window.fetch.bind(window);
  window.fetch = (input, options = {}) => {
    const method = String(
      options.method || input?.method || "GET",
    ).toUpperCase();
    if (!permit(input, method, true)) return Promise.reject(refused());
    return fetch(input, { ...options, redirect: "error" });
  };
  const open = XMLHttpRequest.prototype.open,
    send = XMLHttpRequest.prototype.send;
  XMLHttpRequest.prototype.open = function (method, url, ...rest) {
    method = String(method).toUpperCase();
    if (!permit(url, method, false)) throw refused();
    this.__liveReadRequest1499 = { method, url };
    return open.call(this, method, url, ...rest);
  };
  XMLHttpRequest.prototype.send = function (...args) {
    const p = this.__liveReadRequest1499;
    if (!p || !permit(p.url, p.method, true)) throw refused();
    return send.apply(this, args);
  };
  navigator.sendBeacon = () => {
    state.writes++;
    return false;
  };
  const blockedConstructor = function () {
    state.workers++;
    throw refused();
  };
  for (const name of ["Worker", "SharedWorker", "WebSocket", "EventSource"])
    window[name] = blockedConstructor;
  document.addEventListener(
    "submit",
    (event) => {
      state.writes++;
      event.preventDefault();
    },
    true,
  );
  HTMLFormElement.prototype.submit = function () {
    state.writes++;
    throw refused();
  };
  HTMLFormElement.prototype.requestSubmit = function () {
    state.writes++;
    throw refused();
  };
}
module.exports = { installReadGuard, nativeBlockRules };
