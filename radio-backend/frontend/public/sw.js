const CACHE_NAME = 'rakuraku-v1';

const BYPASS = ['/stream', '/api/', '/admin/download', '/upload'];

self.addEventListener('install', event => {
    event.waitUntil(self.skipWaiting());
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

    const { pathname } = new URL(event.request.url);
    if (BYPASS.some(p => pathname.startsWith(p))) {
        event.respondWith(fetch(event.request));
        return;
    }

    event.respondWith(
        fetch(event.request)
            .then(res => {
                if (res.ok) {
                    const clone = res.clone();
                    caches.open(CACHE_NAME).then(c => c.put(event.request, clone));
                }
                return res;
            })
            .catch(() => caches.match(event.request))
    );
});
