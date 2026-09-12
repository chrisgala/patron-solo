import { test, expect, APIRequestContext } from '@playwright/test';
import { apiContextFor, anonContext, CREATOR } from '../helpers/auth';
import { createPost, createSeries } from '../helpers/fixtures';
import {
  makeVerifiedUser,
  giveSubscription,
  givePurchase,
  getFeedSeriesId,
  cleanupE2eRows,
} from '../helpers/db';

/**
 * Full entitlement matrix, exercised at the API level: each persona fetches
 * each post via GET /api/public/posts/{slug} and we assert granted, reason,
 * and whether content is returned.
 */

interface Persona {
  name: string;
  ctx: APIRequestContext;
}

let personas: Record<string, APIRequestContext>;
let posts: Record<string, { id: string; slug: string }>;
let paidSeries: { id: string; slug: string };

async function fetchPost(ctx: APIRequestContext, slug: string) {
  const res = await ctx.get(`/proxy/api/public/posts/${slug}`);
  return res;
}

test.describe('entitlement gating matrix (API)', () => {
  test.beforeAll(async () => {
    const creator = await apiContextFor(CREATOR.email, CREATOR.password);
    const feedId = await getFeedSeriesId();

    paidSeries = await createSeries(creator, 'gm-series', { priceCents: 2000 });
    posts = {
      tierGated: await createPost(creator, feedId, 'gm-tier', { minTierLevel: 2 }),
      priced: await createPost(creator, feedId, 'gm-priced', { priceCents: 500 }),
      tierPriced: await createPost(creator, feedId, 'gm-tierpriced', {
        minTierLevel: 2,
        priceCents: 900,
      }),
      rolledFree: await createPost(creator, feedId, 'gm-rolled', {
        minTierLevel: 1,
        freeAt: new Date(Date.now() - 3600 * 1000).toISOString(),
      }),
      unpublished: await createPost(creator, feedId, 'gm-unpub', { isPublished: false }),
      inPaidSeries: await createPost(creator, paidSeries.id, 'gm-series-post', {
        minTierLevel: 2,
      }),
    };

    const fanNoSub = await makeVerifiedUser('gm-nosub');
    const fanLow = await makeVerifiedUser('gm-low');
    await giveSubscription(fanLow.id, 1, 'active');
    const fanHigh = await makeVerifiedUser('gm-high');
    await giveSubscription(fanHigh.id, 2, 'active');
    const fanPastDue = await makeVerifiedUser('gm-pastdue');
    await giveSubscription(fanPastDue.id, 2, 'past_due');
    const fanPostBuyer = await makeVerifiedUser('gm-postbuyer');
    await givePurchase(fanPostBuyer.id, { postId: posts.priced.id }, 500);
    await givePurchase(fanPostBuyer.id, { postId: posts.tierPriced.id }, 900);
    const fanSeriesBuyer = await makeVerifiedUser('gm-seriesbuyer');
    await givePurchase(fanSeriesBuyer.id, { seriesId: paidSeries.id }, 2000);

    personas = {
      anon: await anonContext(),
      fanNoSub: await apiContextFor(fanNoSub.email, fanNoSub.password),
      fanLow: await apiContextFor(fanLow.email, fanLow.password),
      fanHigh: await apiContextFor(fanHigh.email, fanHigh.password),
      fanPastDue: await apiContextFor(fanPastDue.email, fanPastDue.password),
      fanPostBuyer: await apiContextFor(fanPostBuyer.email, fanPostBuyer.password),
      fanSeriesBuyer: await apiContextFor(fanSeriesBuyer.email, fanSeriesBuyer.password),
      creator,
    };
  });

  test.afterAll(async () => {
    await cleanupE2eRows();
    for (const ctx of Object.values(personas)) await ctx.dispose();
  });

  async function expectAccess(
    personaName: string,
    slug: string,
    granted: boolean,
    reason: string | null,
  ) {
    const res = await fetchPost(personas[personaName], slug);
    expect(res.ok(), `${personaName} fetching ${slug}`).toBeTruthy();
    const body = await res.json();
    expect(body.access.granted, `${personaName} granted on ${slug}`).toBe(granted);
    expect(body.access.reason, `${personaName} reason on ${slug}`).toBe(reason);
    if (granted) {
      expect(body.content, `${personaName} content on ${slug}`).toBeTruthy();
    } else {
      expect(body.content, `${personaName} content on ${slug}`).toBeNull();
    }
  }

  test('tier-gated post', async () => {
    await expectAccess('anon', posts.tierGated.slug, false, null);
    await expectAccess('fanNoSub', posts.tierGated.slug, false, null);
    await expectAccess('fanLow', posts.tierGated.slug, false, null);
    await expectAccess('fanHigh', posts.tierGated.slug, true, 'subscription');
    await expectAccess('fanPastDue', posts.tierGated.slug, false, null);
    await expectAccess('creator', posts.tierGated.slug, true, 'creator');
  });

  test('priced post', async () => {
    await expectAccess('anon', posts.priced.slug, false, null);
    await expectAccess('fanNoSub', posts.priced.slug, false, null);
    await expectAccess('fanHigh', posts.priced.slug, false, null);
    await expectAccess('fanPostBuyer', posts.priced.slug, true, 'purchase');
    await expectAccess('creator', posts.priced.slug, true, 'creator');
  });

  test('tier-gated and priced post', async () => {
    await expectAccess('anon', posts.tierPriced.slug, false, null);
    await expectAccess('fanLow', posts.tierPriced.slug, false, null);
    await expectAccess('fanHigh', posts.tierPriced.slug, true, 'subscription');
    await expectAccess('fanPostBuyer', posts.tierPriced.slug, true, 'purchase');
    await expectAccess('fanPastDue', posts.tierPriced.slug, false, null);
  });

  test('rolled-free post is free for everyone', async () => {
    for (const persona of ['anon', 'fanNoSub', 'fanLow', 'fanPastDue']) {
      await expectAccess(persona, posts.rolledFree.slug, true, 'free');
    }
    await expectAccess('creator', posts.rolledFree.slug, true, 'creator');
  });

  test('post in a purchasable series unlocks via series purchase', async () => {
    await expectAccess('anon', posts.inPaidSeries.slug, false, null);
    await expectAccess('fanNoSub', posts.inPaidSeries.slug, false, null);
    await expectAccess('fanSeriesBuyer', posts.inPaidSeries.slug, true, 'series');
    await expectAccess('fanHigh', posts.inPaidSeries.slug, true, 'subscription');
    await expectAccess('creator', posts.inPaidSeries.slug, true, 'creator');
  });

  test('unpublished post is 404 on public endpoints for everyone', async () => {
    for (const [name, ctx] of Object.entries(personas)) {
      const res = await fetchPost(ctx, posts.unpublished.slug);
      expect(res.status(), `${name} fetching unpublished post`).toBe(404);
    }
    // and it never appears in the public feed
    const feed = await personas.anon.get('/proxy/api/public/posts?limit=100');
    const slugs = (await feed.json()).map((p: any) => p.slug);
    expect(slugs).not.toContain(posts.unpublished.slug);
  });
});
