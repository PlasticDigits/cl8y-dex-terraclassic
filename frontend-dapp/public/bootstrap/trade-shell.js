;(function () {
  if (!/^\/trade(\/|$)/.test(location.pathname)) return
  var root = document.getElementById('root')
  if (!root) return
  root.innerHTML =
    '<div id="trade-bootstrap-shell" role="status" aria-busy="true" aria-label="Loading trade workspace">' +
    '<div class="trade-bootstrap-title"></div>' +
    '<div class="trade-bootstrap-line"></div>' +
    '<div class="trade-bootstrap-panel"><div class="trade-bootstrap-line" style="width:48px;margin-bottom:8px"></div>' +
    '<div class="trade-bootstrap-line" style="width:100%;max-width:480px"></div></div>' +
    '<div class="trade-bootstrap-grid">' +
    '<div class="trade-bootstrap-block"></div>' +
    '<div class="trade-bootstrap-block"></div>' +
    '<div class="trade-bootstrap-block"></div>' +
    '<div class="trade-bootstrap-block trade-bootstrap-block-tape"></div>' +
    '</div></div>'
})()
