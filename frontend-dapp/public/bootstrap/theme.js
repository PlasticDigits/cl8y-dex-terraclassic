;(function () {
  var s = localStorage.getItem('cl8y-dex-theme')
  if (s !== 'dark' && s !== 'light') {
    s = window.matchMedia('(prefers-color-scheme: light)').matches ? 'light' : 'dark'
  }
  document.documentElement.setAttribute('data-theme', s || 'dark')
})()
