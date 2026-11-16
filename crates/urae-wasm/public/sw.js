// Service Worker for URAE — Atomic Cache-First with Background Network Revalidation
const CACHE_NAME = 'urae-cache-20261116';

// Core assets to pre-cache on install to guarantee complete atomic offline capability
const PRECACHE_ASSETS = [
  './',
  './index.html',
  './style.css',
  './manifest.json',
  './favicon.ico',
  './pkg/urae_wasm.js?v=20261116',
  './pkg/urae_wasm_bg.wasm?v=20261116'
];

// 1. Pre-cache all matching assets on install and activate immediately
self.addEventListener('install', (event) => {
  event.waitUntil(
    caches.open(CACHE_NAME).then((cache) => {
      return cache.addAll(PRECACHE_ASSETS);
    }).then(() => self.skipWaiting())
  );
});

// 2. Purge all legacy caches on activation and claim clients
self.addEventListener('activate', (event) => {
  event.waitUntil(
    caches.keys().then((keys) =>
      Promise.all(
        keys.filter((key) => key !== CACHE_NAME).map((key) => caches.delete(key))
      )
    ).then(() => self.clients.claim())
  );
});

// 3. Fetch Router: Atomic Cache-First Execution
self.addEventListener('fetch', (event) => {
  // Only handle local same-origin GET requests
  if (event.request.method !== 'GET' || !event.request.url.startsWith(self.location.origin)) {
    return;
  }

  const url = new URL(event.request.url);

  // Bypass third-party analytics or beacon endpoints
  if (url.hostname.includes('cloudflareinsights.com') || url.hostname.includes('google-analytics.com')) {
    return;
  }

  // Bypass Edge API endpoints (/api/*)
  if (url.pathname.startsWith('/api/')) {
    return;
  }

  // Never cache the service worker itself so browser can check for updates in the background
  if (url.pathname.endsWith('/sw.js')) {
    event.respondWith(fetch(event.request));
    return;
  }

  // Atomic Cache-First with Background Network Revalidation Strategy:
  // Instantly serve from the active cache bucket so all assets in a session (JS, WASM, HTML)
  // load with zero latency, while double-checking network in background for updates.
  event.respondWith(
    caches.open(CACHE_NAME).then(async (cache) => {
      // 1. Check cache first
      const cachedResponse = await cache.match(event.request);

      // 2. Background network fetch & cache update (double-check network for updated files)
      const networkFetch = fetch(event.request).then((networkResponse) => {
        if (networkResponse && networkResponse.status === 200) {
          cache.put(event.request, networkResponse.clone());
        }
        return networkResponse;
      }).catch(() => null);

      if (cachedResponse) {
        // Cache hit: serve cached response immediately, and revalidate in background
        event.waitUntil(networkFetch);
        return cachedResponse;
      }

      // Cache miss: wait for network response
      const networkResponse = await networkFetch;
      if (networkResponse) {
        return networkResponse;
      }

      // Offline fallback on cache miss
      if (event.request.mode === 'navigate') {
        const fallback = await cache.match('./index.html') || await cache.match('./');
        if (fallback) {
          return fallback;
        }
      }

      return new Response('Offline: Network unavailable', {
        status: 503,
        statusText: 'Service Unavailable',
        headers: new Headers({ 'Content-Type': 'text/plain; charset=utf-8' })
      });
    })
  );
});
