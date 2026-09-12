/* global self, caches, clients, fetch, URL */

const CACHE_NAME = 'patron-v1';

self.addEventListener('install', () => {
  self.skipWaiting();
});

self.addEventListener('activate', (event) => {
  event.waitUntil(
    caches
      .keys()
      .then((keys) =>
        Promise.all(keys.filter((key) => key !== CACHE_NAME).map((key) => caches.delete(key))),
      )
      .then(() => self.clients.claim()),
  );
});

self.addEventListener('fetch', (event) => {
  const request = event.request;
  if (request.method !== 'GET') {
    return;
  }

  const url = new URL(request.url);

  // Cache-first for hashed build assets
  if (url.origin === self.location.origin && url.pathname.startsWith('/assets/')) {
    event.respondWith(
      caches.match(request).then(
        (cached) =>
          cached ||
          fetch(request).then((response) => {
            const copy = response.clone();
            if (response.ok) {
              caches.open(CACHE_NAME).then((cache) => cache.put(request, copy));
            }
            return response;
          }),
      ),
    );
    return;
  }

  // Network-first with cache fallback for page navigations
  if (request.mode === 'navigate') {
    event.respondWith(
      fetch(request)
        .then((response) => {
          const copy = response.clone();
          if (response.ok) {
            caches.open(CACHE_NAME).then((cache) => cache.put(request, copy));
          }
          return response;
        })
        .catch(() => caches.match(request)),
    );
  }
});

self.addEventListener('push', (event) => {
  // Payload-less push: look up the newest post and notify about it. The app
  // normally reaches the backend through the /proxy prefix; fall back to a
  // direct path for deployments that serve the API on the same origin.
  event.waitUntil(
    fetch('/proxy/api/public/posts?limit=1', { credentials: 'include' })
      .then((response) =>
        response.ok ? response : fetch('/api/public/posts?limit=1', { credentials: 'include' }),
      )
      .then((response) => (response.ok ? response.json() : []))
      .then((posts) => {
        const post = Array.isArray(posts) && posts.length > 0 ? posts[0] : null;
        return self.registration.showNotification(post ? post.title : 'New post', {
          body: post ? 'A new post was just published.' : 'Something new was just published.',
          icon: '/favicon-96x96.png',
          data: { url: post ? `/posts/${post.slug}` : '/' },
        });
      })
      .catch(() =>
        self.registration.showNotification('New post', {
          body: 'Something new was just published.',
          icon: '/favicon-96x96.png',
          data: { url: '/' },
        }),
      ),
  );
});

self.addEventListener('notificationclick', (event) => {
  event.notification.close();
  const targetUrl = (event.notification.data && event.notification.data.url) || '/';
  event.waitUntil(clients.openWindow(targetUrl));
});
