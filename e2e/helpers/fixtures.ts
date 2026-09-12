import { APIRequestContext } from '@playwright/test';
import { randomUUID } from 'node:crypto';
import { E2E_PREFIX } from './db';

/** Random-but-unique post number so parallel fixtures never collide. */
function uniqueNumber(): number {
  return 100000 + Math.floor(Math.random() * 800000);
}

export function uniqueSlug(tag: string): string {
  return `${E2E_PREFIX}${tag}-${randomUUID().slice(0, 8)}`;
}

export interface PostFixtureOptions {
  title?: string;
  content?: string;
  minTierLevel?: number | null;
  priceCents?: number | null;
  freeAt?: string | null;
  isPublished?: boolean;
  kind?: string;
  audioFileId?: string;
  videoFileId?: string;
  imageFileIds?: string[];
}

/** Creates a post via the creator API in the given series. */
export async function createPost(
  creatorCtx: APIRequestContext,
  seriesId: string,
  tag: string,
  opts: PostFixtureOptions = {},
): Promise<{ id: string; slug: string }> {
  const slug = uniqueSlug(tag);
  const res = await creatorCtx.post('/proxy/api/posts', {
    data: {
      seriesId,
      title: opts.title ?? slug,
      content: opts.content ?? `<p>content of ${slug}</p>`,
      slug,
      postNumber: uniqueNumber(),
      isPublished: opts.isPublished ?? true,
      kind: opts.kind ?? 'article',
      minTierLevel: opts.minTierLevel ?? null,
      priceCents: opts.priceCents ?? null,
      freeAt: opts.freeAt ?? null,
      audioFileId: opts.audioFileId,
      videoFileId: opts.videoFileId,
      imageFileIds: opts.imageFileIds,
    },
  });
  if (!res.ok()) {
    throw new Error(`createPost failed: ${res.status()} ${await res.text()}`);
  }
  const body = await res.json();
  return { id: body.id, slug };
}

/** Creates a tier via the creator API. Level should be unique (use 90-99). */
export async function createTier(
  creatorCtx: APIRequestContext,
  level: number,
  priceCents: number,
  tag = 'tier',
): Promise<{ id: string; name: string }> {
  const name = `${E2E_PREFIX}${tag}-${randomUUID().slice(0, 6)}`;
  const res = await creatorCtx.post('/proxy/api/tiers', {
    data: { name, description: null, level, priceCents },
  });
  if (!res.ok()) {
    throw new Error(`createTier failed: ${res.status()} ${await res.text()}`);
  }
  const body = await res.json();
  return { id: body.id, name };
}

/** Creates a (non-feed) series via the creator API. */
export async function createSeries(
  creatorCtx: APIRequestContext,
  tag: string,
  opts: { priceCents?: number | null } = {},
): Promise<{ id: string; slug: string }> {
  const slug = uniqueSlug(tag);
  const res = await creatorCtx.post('/proxy/api/series', {
    data: { title: slug, slug, description: null, priceCents: opts.priceCents ?? null },
  });
  if (!res.ok()) {
    throw new Error(`createSeries failed: ${res.status()} ${await res.text()}`);
  }
  const body = await res.json();
  return { id: body.id, slug };
}
