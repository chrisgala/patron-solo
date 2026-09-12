/**
 * Typed fetch wrapper for the single-creator endpoints that are not part of
 * the published patronts SDK. Uses the same base URL as `patronClient` in
 * `src/lib/utils.ts` and always sends cookies.
 */

const API_BASE: string = import.meta.env.VITE_SERVER_URL || 'http://localhost:8080';

/** Content kind of a post */
export type PostKind = 'update' | 'article' | 'audio' | 'video' | 'images';

/** Why access to a post was granted */
export type AccessReason = 'free' | 'subscription' | 'purchase' | 'series' | 'creator';

/** Access metadata attached to a public post */
export interface PostAccess {
  granted: boolean;
  reason: AccessReason | null;
  requiredTierLevel: number | null;
  priceCents: number | null;
  freeAt: string | null;
}

/** A post as seen by the public site */
export interface PublicPostResponse {
  id: string;
  seriesId: string;
  title: string;
  kind: PostKind;
  slug: string;
  postNumber: number;
  thumbnailUrl: string | null;
  createdAt: string | null;
  access: PostAccess;
  content: string | null;
  audioFileId: string | null;
  videoFileId: string | null;
  imageFileIds: string[] | null;
}

/** Public creator profile */
export interface CreatorProfile {
  displayName: string | null;
  avatarUrl: string | null;
  banner: string | null;
  description: string | null;
}

/** Response of GET /api/public/site */
export interface SiteResponse {
  creator: CreatorProfile | null;
}

/** A membership tier */
export interface TierResponse {
  id: string;
  name: string;
  description: string | null;
  level: number;
  priceCents: number;
  currency: string;
  isActive: boolean;
  createdAt: string | null;
}

/** Request body for creating a tier */
export interface CreateTierRequest {
  name: string;
  description?: string | null;
  level: number;
  priceCents: number;
}

/** Request body for updating a tier */
export interface UpdateTierRequest {
  name?: string;
  description?: string | null;
  level?: number;
  priceCents?: number;
  isActive?: boolean;
}

/** A series as seen by the public site */
export interface PublicSeriesResponse {
  id: string;
  title: string;
  description?: string | null;
  slug?: string;
  coverImageUrl?: string | null;
  priceCents?: number | null;
  minTierLevel?: number | null;
  isFeed?: boolean;
  owned: boolean;
  [key: string]: unknown;
}

/** Active subscription state for the current user */
export interface BillingSubscription {
  tierId: string;
  tierLevel: number;
  status: string;
  cancelAtPeriodEnd: boolean;
  currentPeriodEnd: string | null;
}

/** A one-off purchase made by the current user */
export interface BillingPurchase {
  postId: string | null;
  seriesId: string | null;
  amountCents: number;
  createdAt: string | null;
}

/** Response of GET /api/billing/me */
export interface BillingMeResponse {
  subscription: BillingSubscription | null;
  purchases: BillingPurchase[];
}

/** Response containing a Stripe redirect URL */
export interface CheckoutUrlResponse {
  url: string;
}

/** Request body for creating a post via /api/posts */
export interface CreatePostPayload {
  seriesId: string;
  title: string;
  content: string;
  slug: string;
  postNumber: number;
  isPublished?: boolean | null;
  thumbnailUrl?: string | null;
  audioFileId?: string | null;
  videoFileId?: string | null;
  kind?: PostKind | null;
  minTierLevel?: number | null;
  priceCents?: number | null;
  freeAt?: string | null;
  imageFileIds?: string[] | null;
}

/** Request body for updating a post via /api/posts/:id */
export interface UpdatePostPayload {
  seriesId?: string | null;
  title?: string;
  content?: string;
  slug?: string;
  postNumber?: number;
  isPublished?: boolean | null;
  thumbnailUrl?: string | null;
  audioFileId?: string | null;
  videoFileId?: string | null;
  kind?: PostKind | null;
  minTierLevel?: number | null;
  clearMinTier?: boolean;
  priceCents?: number | null;
  clearPrice?: boolean;
  freeAt?: string | null;
  clearFreeAt?: boolean;
  imageFileIds?: string[] | null;
}

/** Options for a JSON API request */
interface RequestOptions {
  path: string;
  method?: string;
  body?: unknown;
}

/** Error thrown for non-2xx API responses */
export class ApiError extends Error {
  /** HTTP status code of the failed response */
  public status: number;

  /**
   * Creates an ApiError.
   *
   * @param {object} params - Error details
   * @param {number} params.status - HTTP status code
   * @param {string} params.message - Error message
   */
  constructor({ status, message }: { status: number; message: string }) {
    super(message);
    this.name = 'ApiError';
    this.status = status;
  }
}

/**
 * Performs a JSON request against the API with credentials included.
 *
 * @param {RequestOptions} options - Path, method and optional JSON body
 * @returns {Promise<T>} Parsed JSON response (undefined for 204 responses)
 */
const request = async <T>({ path, method = 'GET', body }: RequestOptions): Promise<T> => {
  const response = await fetch(`${API_BASE}${path}`, {
    method,
    credentials: 'include',
    headers: body !== undefined ? { 'Content-Type': 'application/json' } : undefined,
    body: body !== undefined ? JSON.stringify(body) : undefined,
  });

  if (!response.ok) {
    let message = `Request failed with status ${response.status}`;
    try {
      const errorBody = (await response.json()) as { error?: string; message?: string };
      message = errorBody.error || errorBody.message || message;
    } catch {
      // Non-JSON error body; keep default message
    }
    throw new ApiError({ status: response.status, message });
  }

  if (response.status === 204) {
    return undefined as T;
  }

  return (await response.json()) as T;
};

/**
 * Fetches the public site info (creator profile).
 *
 * @returns {Promise<SiteResponse>} The site response
 */
export const getSite = (): Promise<SiteResponse> => request({ path: '/api/public/site' });

/**
 * Fetches the public post feed, newest first.
 *
 * @param {object} [params] - Optional filters
 * @param {string} [params.seriesId] - Only posts from this series
 * @param {number} [params.offset] - Pagination offset
 * @param {number} [params.limit] - Page size
 * @returns {Promise<PublicPostResponse[]>} The posts
 */
export const getPublicPosts = (params?: {
  seriesId?: string;
  offset?: number;
  limit?: number;
}): Promise<PublicPostResponse[]> => {
  const query = new URLSearchParams();
  if (params?.seriesId) query.set('seriesId', params.seriesId);
  if (params?.offset !== undefined) query.set('offset', String(params.offset));
  if (params?.limit !== undefined) query.set('limit', String(params.limit));
  const qs = query.toString();
  return request({ path: `/api/public/posts${qs ? `?${qs}` : ''}` });
};

/**
 * Fetches a single public post by id or slug.
 *
 * @param {string} idOrSlug - The post id or slug
 * @returns {Promise<PublicPostResponse>} The post
 */
export const getPublicPost = (idOrSlug: string): Promise<PublicPostResponse> =>
  request({ path: `/api/public/posts/${encodeURIComponent(idOrSlug)}` });

/**
 * Fetches the publicly joinable tiers.
 *
 * @returns {Promise<TierResponse[]>} Active tiers ordered by level
 */
export const getPublicTiers = (): Promise<TierResponse[]> =>
  request({ path: '/api/public/tiers' });

/**
 * Fetches the public series list.
 *
 * @returns {Promise<PublicSeriesResponse[]>} Visible series
 */
export const getPublicSeries = (): Promise<PublicSeriesResponse[]> =>
  request({ path: '/api/public/series' });

/**
 * Fetches a single public series by id or slug.
 *
 * @param {string} idOrSlug - The series id or slug
 * @returns {Promise<PublicSeriesResponse>} The series
 */
export const getPublicSeriesItem = (idOrSlug: string): Promise<PublicSeriesResponse> =>
  request({ path: `/api/public/series/${encodeURIComponent(idOrSlug)}` });

/**
 * Starts a Stripe checkout for a tier subscription.
 *
 * @param {string} tierId - The tier to subscribe to
 * @returns {Promise<CheckoutUrlResponse>} The Stripe checkout URL
 */
export const subscribe = (tierId: string): Promise<CheckoutUrlResponse> =>
  request({ path: '/api/billing/subscribe', method: 'POST', body: { tierId } });

/**
 * Starts a Stripe checkout for a one-off post or series purchase.
 *
 * @param {object} target - The purchase target
 * @param {string} [target.postId] - Post to buy
 * @param {string} [target.seriesId] - Series to buy
 * @returns {Promise<CheckoutUrlResponse>} The Stripe checkout URL
 */
export const purchase = (target: {
  postId?: string;
  seriesId?: string;
}): Promise<CheckoutUrlResponse> =>
  request({ path: '/api/billing/purchase', method: 'POST', body: target });

/**
 * Opens the Stripe billing portal for the current user.
 *
 * @returns {Promise<CheckoutUrlResponse>} The billing portal URL
 */
export const portal = (): Promise<CheckoutUrlResponse> =>
  request({ path: '/api/billing/portal', method: 'POST' });

/**
 * Fetches the current user's subscription and purchases.
 *
 * @returns {Promise<BillingMeResponse>} The billing state
 */
export const getBillingMe = (): Promise<BillingMeResponse> => request({ path: '/api/billing/me' });

/**
 * Fetches all tiers (creator only; includes inactive tiers).
 *
 * @returns {Promise<TierResponse[]>} All tiers ordered by level
 */
export const getTiers = (): Promise<TierResponse[]> => request({ path: '/api/tiers' });

/**
 * Creates a tier (creator only).
 *
 * @param {CreateTierRequest} body - The tier to create
 * @returns {Promise<TierResponse>} The created tier
 */
export const createTier = (body: CreateTierRequest): Promise<TierResponse> =>
  request({ path: '/api/tiers', method: 'POST', body });

/**
 * Updates a tier (creator only).
 *
 * @param {object} params - Update parameters
 * @param {string} params.tierId - The tier id
 * @param {UpdateTierRequest} params.body - Fields to update
 * @returns {Promise<TierResponse>} The updated tier
 */
export const updateTier = (params: {
  tierId: string;
  body: UpdateTierRequest;
}): Promise<TierResponse> =>
  request({ path: `/api/tiers/${params.tierId}`, method: 'PUT', body: params.body });

/**
 * Deletes a tier (creator only).
 *
 * @param {string} tierId - The tier id
 * @returns {Promise<void>} Resolves when deleted
 */
export const deleteTier = (tierId: string): Promise<void> =>
  request({ path: `/api/tiers/${tierId}`, method: 'DELETE' });

/**
 * Creates a post via the raw API (supports the new gating fields).
 *
 * @param {CreatePostPayload} body - The post payload
 * @returns {Promise<unknown>} The created post
 */
export const createPost = (body: CreatePostPayload): Promise<unknown> =>
  request({ path: '/api/posts', method: 'POST', body });

/**
 * Updates a post via the raw API (supports the new gating fields).
 *
 * @param {object} params - Update parameters
 * @param {string} params.postId - The post id
 * @param {UpdatePostPayload} params.body - Fields to update
 * @returns {Promise<unknown>} The updated post
 */
export const updatePost = (params: {
  postId: string;
  body: UpdatePostPayload;
}): Promise<unknown> =>
  request({ path: `/api/posts/${params.postId}`, method: 'PUT', body: params.body });

/**
 * Fetches the web-push VAPID public key. Rejects with a 404 ApiError when
 * push is not configured on the server.
 *
 * @returns {Promise<{publicKey: string}>} The public key
 */
export const pushKey = (): Promise<{ publicKey: string }> =>
  request({ path: '/api/public/push/key' });

/**
 * Registers a push subscription with the server.
 *
 * @param {object} sub - The subscription keys
 * @param {string} sub.endpoint - Push endpoint URL
 * @param {string} sub.p256dh - Client public key
 * @param {string} sub.auth - Auth secret
 * @returns {Promise<void>} Resolves when stored
 */
export const pushSubscribe = (sub: {
  endpoint: string;
  p256dh: string;
  auth: string;
}): Promise<void> => request({ path: '/api/push/subscribe', method: 'POST', body: sub });

/**
 * Removes a push subscription from the server.
 *
 * @param {string} endpoint - The subscription endpoint URL
 * @returns {Promise<void>} Resolves when removed
 */
export const pushUnsubscribe = (endpoint: string): Promise<void> =>
  request({ path: '/api/push/subscribe', method: 'DELETE', body: { endpoint } });

/**
 * Builds the URL for streaming a media file from the entitlement-enforced CDN
 * endpoint. Cookies are sent automatically for same-origin media elements.
 *
 * @param {string} fileId - The file id
 * @returns {string} The CDN URL for the file
 */
export const cdnFileUrl = (fileId: string): string => `${API_BASE}/api/cdn/files/${fileId}`;
