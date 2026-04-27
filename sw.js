// 当前版本 - 修改此值以强制更新service worker
const CACHE_VERSION = '1.0.0';
const CACHE_NAME = `rakuraku-music-cache-${CACHE_VERSION}`;

// 需要缓存的资源
const CACHE_URLS = [
  '/',
  '/admin',
  '/login.html',
  '/manifest.json',
  '/pwa-icon-192.png',
  '/pwa-icon-512.png',
  // 其他静态资源将由动态缓存处理
];

// 安装事件 - 缓存基本资源
self.addEventListener('install', event => {
  console.log('[Service Worker] 安装中...');

  event.waitUntil(
    caches.open(CACHE_NAME)
      .then(cache => {
        console.log('[Service Worker] 缓存基本资源...');
        return cache.addAll(CACHE_URLS);
      })
      .then(() => {
        console.log('[Service Worker] 安装完成');
        return self.skipWaiting();
      })
  );
});

// 激活事件 - 清理旧缓存
self.addEventListener('activate', event => {
  console.log('[Service Worker] 激活中...');

  event.waitUntil(
    caches.keys().then(cacheNames => {
      return Promise.all(
        cacheNames.map(cacheName => {
          if (cacheName !== CACHE_NAME) {
            console.log('[Service Worker] 删除旧缓存:', cacheName);
            return caches.delete(cacheName);
          }
        })
      );
    }).then(() => {
      console.log('[Service Worker] 激活完成');
      return self.clients.claim();
    })
  );
});

// 拦截请求 - 网络优先，回退缓存策略
self.addEventListener('fetch', event => {
  const { request } = event;

  // 音频流特殊处理 - 不缓存流媒体
  if (request.url.includes('/stream') || request.url.includes('/api/')) {
    // API和流媒体使用网络优先，不缓存
    event.respondWith(fetch(request));
    return;
  }

  // 其他资源使用缓存优先策略
  event.respondWith(
    caches.match(request)
      .then(cachedResponse => {
        if (cachedResponse) {
          // 有缓存，同时更新缓存（后台）
          event.waitUntil(
            fetch(request).then(response => {
              if (response.ok) {
                caches.open(CACHE_NAME).then(cache => {
                  cache.put(request, response.clone());
                });
              }
              return response;
            }).catch(() => cachedResponse)
          );
          return cachedResponse;
        }

        // 无缓存，发起网络请求
        return fetch(request)
          .then(response => {
            // 只缓存成功的响应
            if (response.ok) {
              const responseClone = response.clone();
              caches.open(CACHE_NAME).then(cache => {
                cache.put(request, responseClone);
              });
            }
            return response;
          })
          .catch(error => {
            console.error('[Service Worker] 网络请求失败:', error);

            // 尝试返回离线页面
            if (request.headers.get('Accept').includes('text/html')) {
              return caches.match('/');
            }

            // 返回自定义错误响应
            return new Response(
              JSON.stringify({
                error: '网络连接失败',
                message: '请检查网络连接或稍后重试'
              }),
              {
                status: 503,
                statusText: 'Service Unavailable',
                headers: { 'Content-Type': 'application/json' }
              }
            );
          });
      })
  );
});

// 处理推送通知
self.addEventListener('push', event => {
  const data = event.data ? event.data.json() : {};
  const title = data.title || 'Rakuraku Music';
  const options = {
    body: data.body || '电台有新的通知',
    icon: '/pwa-icon-192.png',
    badge: '/pwa-icon-192.png',
    vibrate: [100, 50, 100],
    data: data.data || {},
    actions: data.actions || []
  };

  event.waitUntil(
    self.registration.showNotification(title, options)
  );
});

// 处理通知点击
self.addEventListener('notificationclick', event => {
  const { notification } = event;
  notification.close();

  const urlToOpen = '/';

  event.waitUntil(
    clients.matchAll({
      type: 'window',
      includeUncontrolled: true
    }).then(windowClients => {
      // 检查是否有打开的窗口
      for (const client of windowClients) {
        if (client.url === urlToOpen && 'focus' in client) {
          return client.focus();
        }
      }

      // 没有找到相关窗口，打开新窗口
      if (clients.openWindow) {
        return clients.openWindow(urlToOpen);
      }
    })
  );
});

// 处理后台同步
self.addEventListener('sync', event => {
  console.log('[Service Worker] 后台同步事件:', event.tag);
});