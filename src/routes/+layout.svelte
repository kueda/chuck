<script lang="ts">
import '../app.css';

const { children } = $props();

// I haven't found a way to test the `csp` config from tauri.conf.json in a
// dev env (there's a devCsp config property that doesn't work with vite),
// and to make things more annoying, there are some dev-only CSP allowances
// that are necessary, so... this is a way to test CSP config in dev with
// the huge caveat that prodCsp here must be manually kept in sync with the
// `csp` config from tauri.conf.json
const connectSrc = [
  "'self'",
  'basemap:',
  'http://ipc.localhost',
  'http://tiles.localhost',
  'https://api.inaturalist.org:*',
  'https://protomaps.github.io:*',
  'https://tile.openstreetmap.org:*',
  'ipc:',
  'tiles:',
].join(' ');
const devCsp = {
  'script-src': "'self' 'unsafe-inline' http://localhost:*",
  'style-src': "'self' 'unsafe-inline' http://localhost:*",
  'connect-src': `${connectSrc} ws://localhost:* http://localhost:*`,
};
const prodCsp = {
  'default-src': "'self' tiles: http://tiles.localhost",
  'img-src': "'self' data: blob: asset: tiles: *",
  'connect-src': connectSrc,
  'object-src': "'none'",
  'frame-src': "'self'",
  'worker-src': 'blob:',
};
const csp = import.meta.env.DEV
  ? Object.entries({ ...prodCsp, ...devCsp })
      .map((pair) => `${pair[0]} ${pair[1]}`)
      .join('; ')
  : Object.entries(prodCsp)
      .map((pair) => `${pair[0]} ${pair[1]}`)
      .join('; ');
</script>

<svelte:head>
  <meta http-equiv="content-security-policy" content={csp} />
</svelte:head>

{@render children()}
