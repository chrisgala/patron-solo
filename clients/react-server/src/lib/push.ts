import { pushKey, pushSubscribe, pushUnsubscribe } from '@/lib/api';

/**
 * Converts a base64url-encoded VAPID key into the byte array expected by
 * PushManager.subscribe.
 *
 * @param {string} base64String - The base64url-encoded key
 * @returns {Uint8Array} The decoded bytes
 */
const urlBase64ToUint8Array = (base64String: string): Uint8Array => {
  const padding = '='.repeat((4 - (base64String.length % 4)) % 4);
  const base64 = (base64String + padding).replace(/-/g, '+').replace(/_/g, '/');
  const rawData = window.atob(base64);
  const outputArray = new Uint8Array(rawData.length);
  for (let i = 0; i < rawData.length; i += 1) {
    outputArray[i] = rawData.charCodeAt(i);
  }
  return outputArray;
};

/**
 * Registers the service worker at /sw.js. Safe to call multiple times.
 *
 * @returns {Promise<void>} Resolves when registration is attempted
 */
export const registerServiceWorker = async (): Promise<void> => {
  if (typeof navigator === 'undefined' || !('serviceWorker' in navigator)) {
    return;
  }
  try {
    await navigator.serviceWorker.register('/sw.js');
  } catch (error) {
    console.warn('Service worker registration failed:', error);
  }
};

/**
 * Checks whether push notifications can be offered in this browser and are
 * configured on the server.
 *
 * @returns {Promise<boolean>} True when the push bell should be shown
 */
export const isPushSupported = async (): Promise<boolean> => {
  if (
    typeof window === 'undefined' ||
    !('serviceWorker' in navigator) ||
    !('PushManager' in window) ||
    !('Notification' in window)
  ) {
    return false;
  }
  try {
    await pushKey();
    return true;
  } catch {
    return false;
  }
};

/**
 * Checks whether the current browser already has an active push subscription.
 *
 * @returns {Promise<boolean>} True when subscribed
 */
export const isPushEnabled = async (): Promise<boolean> => {
  if (typeof navigator === 'undefined' || !('serviceWorker' in navigator)) {
    return false;
  }
  const registration = await navigator.serviceWorker.getRegistration();
  const subscription = await registration?.pushManager.getSubscription();
  return Boolean(subscription);
};

/**
 * Subscribes the browser to push notifications and registers the
 * subscription with the server.
 *
 * @returns {Promise<boolean>} True when the subscription is active
 */
export const enablePush = async (): Promise<boolean> => {
  const permission = await Notification.requestPermission();
  if (permission !== 'granted') {
    return false;
  }

  let registration = await navigator.serviceWorker.getRegistration();
  if (!registration) {
    await registerServiceWorker();
    registration = await navigator.serviceWorker.ready;
  }

  const { publicKey } = await pushKey();
  const subscription = await registration.pushManager.subscribe({
    userVisibleOnly: true,
    applicationServerKey: urlBase64ToUint8Array(publicKey) as BufferSource,
  });

  const json = subscription.toJSON();
  if (!json.endpoint || !json.keys?.p256dh || !json.keys?.auth) {
    return false;
  }

  await pushSubscribe({
    endpoint: json.endpoint,
    p256dh: json.keys.p256dh,
    auth: json.keys.auth,
  });
  return true;
};

/**
 * Unsubscribes the browser from push notifications and removes the
 * subscription from the server.
 *
 * @returns {Promise<void>} Resolves when unsubscribed
 */
export const disablePush = async (): Promise<void> => {
  const registration = await navigator.serviceWorker.getRegistration();
  const subscription = await registration?.pushManager.getSubscription();
  if (!subscription) {
    return;
  }
  const endpoint = subscription.endpoint;
  await subscription.unsubscribe();
  try {
    await pushUnsubscribe(endpoint);
  } catch (error) {
    console.warn('Failed to remove push subscription from server:', error);
  }
};
