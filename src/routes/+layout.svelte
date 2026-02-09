<script lang="ts">
import '../app.css';
import tauriConf from '../../src-tauri/tauri.conf.json';

const { children } = $props();

// Test the `csp` config from tauri.conf.json in a dev env (there's a devCsp
// config property that doesn't work with vite). There are some dev-only CSP
// allowances that are necessary.
const connectSrc = tauriConf.app.security.csp['connect-src'];
const devCsp = {
  'script-src': "'self' 'unsafe-inline' http://localhost:*",
  'style-src': "'self' 'unsafe-inline' http://localhost:*",
  'connect-src': `${connectSrc} ws://localhost:* http://localhost:*`,
};
const prodCsp = {
  'default-src': tauriConf.app.security.csp['default-src'],
  'img-src': tauriConf.app.security.csp['img-src'],
  'connect-src': connectSrc,
  'object-src': tauriConf.app.security.csp['object-src'],
  'frame-src': tauriConf.app.security.csp['frame-src'],
  'worker-src': tauriConf.app.security.csp['worker-src'],
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
