const CACHE_NAME = 'rakuraku-v3';
const SCOPE_URL = new URL(self.registration.scope);

const PRECACHE = [
  '.',
  'index.html',
  'icon.svg',
  'icon-192.png',
  'icon-512.png',
  'manifest.json'
].map(path => new URL(path, SCOPE_URL).toString());

const BYPASS = ['/stream', '/api/', '/ws'];

function scopedPath(url) {
  let path = url.pathname;
  const scopePath = SCOPE_URL.pathname.endsWith('/') ? SCOPE_URL.pathname : SCOPE_URL.pathname + '/';
  if (scopePath !== '/' && path.startsWith(scopePath)) {
    path = '/' + path.slice(scopePath.length);
  }
  return path;
}

self.addEventListener('install', event => {
  event.waitUntil(
    caches.open(CACHE_NAME)
      .then(cache => Promise.allSettled(PRECACHE.map(url => cache.add(url))))
      .then(() => self.skipWaiting())
  );
});

self.addEventListener('activate', event => {
  event.waitUntil(
    caches.keys()
      .then(keys => Promise.all(
        keys.filter(k => k !== CACHE_NAME).map(k => caches.delete(k))
      ))
      .then(() => self.clients.claim())
  );
});

self.addEventListener('fetch', event => {
  if (event.request.method !== 'GET') return;

  const requestUrl = new URL(event.request.url);
  if (requestUrl.origin !== SCOPE_URL.origin) return;

  const pathname = scopedPath(requestUrl);
  if (BYPASS.some(p => pathname.startsWith(p))) {
    event.respondWith(fetch(event.request));
    return;
  }

  if (event.request.mode === 'navigate') {
    event.respondWith(
      fetch(event.request)
        .then(res => {
          if (res.ok) {
            const clone = res.clone();
            caches.open(CACHE_NAME).then(cache => cache.put(event.request, clone));
          }
          return res;
        })
        .catch(() => caches.match(event.request).then(cached => cached || caches.match(new URL('index.html', SCOPE_URL).toString())))
    );
    return;
  }

  event.respondWith(
    caches.match(event.request)
      .then(cached => {
        const fetchPromise = fetch(event.request)
          .then(res => {
            if (res.ok) {
              const clone = res.clone();
              caches.open(CACHE_NAME).then(c => c.put(event.request, clone));
            }
            return res;
          })
          .catch(() => cached);
        return cached || fetchPromise;
      })
  );
});
