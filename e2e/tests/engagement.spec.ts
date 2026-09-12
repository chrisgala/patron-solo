import { test, expect, APIRequestContext } from '@playwright/test';
import { BACKEND_URL } from '../playwright.config';
import { apiContextFor, anonContext, loginViaApi, CREATOR } from '../helpers/auth';
import { createPost } from '../helpers/fixtures';
import {
  makeVerifiedUser,
  giveSubscription,
  getFeedSeriesId,
  cleanupE2eRows,
  TestUser,
} from '../helpers/db';

let creatorCtx: APIRequestContext;
let anon: APIRequestContext;
let entitled: APIRequestContext;
let entitled2: APIRequestContext;
let plainFan: APIRequestContext;
let plainFanUser: TestUser;
let freePost: { id: string; slug: string };
let gatedPost: { id: string; slug: string };

test.describe('comments and likes', () => {
  test.beforeAll(async () => {
    creatorCtx = await apiContextFor(CREATOR.email, CREATOR.password);
    anon = await anonContext();
    const feedId = await getFeedSeriesId();
    freePost = await createPost(creatorCtx, feedId, 'eng-free', {
      content: '<p>free engagement post</p>',
    });
    gatedPost = await createPost(creatorCtx, feedId, 'eng-gated', { minTierLevel: 2 });

    const fanA = await makeVerifiedUser('eng-sub');
    await giveSubscription(fanA.id, 2, 'active');
    entitled = await apiContextFor(fanA.email, fanA.password);

    const fanB = await makeVerifiedUser('eng-sub2');
    await giveSubscription(fanB.id, 2, 'active');
    entitled2 = await apiContextFor(fanB.email, fanB.password);

    plainFanUser = await makeVerifiedUser('eng-plain');
    plainFan = await apiContextFor(plainFanUser.email, plainFanUser.password);
  });

  test.afterAll(async () => {
    await cleanupE2eRows();
    for (const ctx of [creatorCtx, anon, entitled, entitled2, plainFan]) await ctx.dispose();
  });

  test('listing comments respects post entitlement', async () => {
    // Free post: everyone can list
    const anonFree = await anon.get(`${BACKEND_URL}/api/public/posts/${freePost.id}/comments`);
    expect(anonFree.status()).toBe(200);
    expect(Array.isArray(await anonFree.json())).toBe(true);

    // Gated post: 403 for anonymous and non-entitled fans, 200 for entitled/creator
    expect((await anon.get(`${BACKEND_URL}/api/public/posts/${gatedPost.id}/comments`)).status()).toBe(403);
    expect(
      (await plainFan.get(`${BACKEND_URL}/api/public/posts/${gatedPost.id}/comments`)).status(),
    ).toBe(403);
    expect(
      (await entitled.get(`${BACKEND_URL}/api/public/posts/${gatedPost.id}/comments`)).status(),
    ).toBe(200);
    expect(
      (await creatorCtx.get(`${BACKEND_URL}/api/public/posts/${gatedPost.id}/comments`)).status(),
    ).toBe(200);
  });

  test('creating comments: auth, entitlement, replies', async () => {
    // Anonymous -> 401
    const anonRes = await anon.post(`${BACKEND_URL}/api/posts/${freePost.id}/comments`, {
      data: { content: 'anon comment' },
    });
    expect(anonRes.status()).toBe(401);

    // Non-entitled fan on a gated post -> 403
    const plainRes = await plainFan.post(`${BACKEND_URL}/api/posts/${gatedPost.id}/comments`, {
      data: { content: 'should not land' },
    });
    expect(plainRes.status()).toBe(403);

    // Entitled fan -> 201, and the comment shows up in the list
    const created = await entitled.post(`${BACKEND_URL}/api/posts/${gatedPost.id}/comments`, {
      data: { content: 'e2e top-level comment' },
    });
    expect(created.status()).toBe(201);
    const parent = await created.json();
    expect(parent.content).toBe('e2e top-level comment');
    expect(parent.parentId).toBeNull();
    expect(parent.canDelete).toBe(true);

    // Reply to it
    const reply = await entitled2.post(`${BACKEND_URL}/api/posts/${gatedPost.id}/comments`, {
      data: { content: 'e2e reply', parentId: parent.id },
    });
    expect(reply.status()).toBe(201);
    expect((await reply.json()).parentId).toBe(parent.id);

    const list = await entitled.get(`${BACKEND_URL}/api/public/posts/${gatedPost.id}/comments`);
    const comments = await list.json();
    expect(comments.some((c: any) => c.id === parent.id)).toBe(true);
    expect(comments.some((c: any) => c.parentId === parent.id)).toBe(true);

    // Reply to a nonexistent parent -> 404
    const badParent = await entitled.post(`${BACKEND_URL}/api/posts/${gatedPost.id}/comments`, {
      data: { content: 'orphan', parentId: '00000000-0000-0000-0000-000000000000' },
    });
    expect(badParent.status()).toBe(404);
  });

  test('over-long comment (>5000 chars) is rejected with a 4xx', async () => {
    // BUG: the backend rejects the comment but maps the validation error to
    // ServiceError::Unknown, which surfaces as HTTP 500
    // ("Unknown error: Comment must be between 1 and 5000 characters").
    // A validation failure should be a 400. Repro:
    //   POST /api/posts/{id}/comments with 5001 chars of content -> 500.
    test.fixme();

    const res = await entitled.post(`${BACKEND_URL}/api/posts/${freePost.id}/comments`, {
      data: { content: 'x'.repeat(5001) },
    });
    expect(res.status()).toBe(400);
  });

  test('deleting comments: author yes, creator yes, another fan no', async () => {
    const mk = async (): Promise<string> => {
      const res = await entitled.post(`${BACKEND_URL}/api/posts/${freePost.id}/comments`, {
        data: { content: 'deletable comment' },
      });
      expect(res.status()).toBe(201);
      return (await res.json()).id;
    };

    // Author deletes own comment
    const own = await mk();
    expect((await entitled.delete(`${BACKEND_URL}/api/comments/${own}`)).status()).toBe(204);

    // Another fan cannot delete it
    const foreign = await mk();
    expect((await entitled2.delete(`${BACKEND_URL}/api/comments/${foreign}`)).status()).toBe(403);

    // The creator can moderate any comment
    expect((await creatorCtx.delete(`${BACKEND_URL}/api/comments/${foreign}`)).status()).toBe(204);

    // Deleted comments disappear from the list
    const list = await entitled.get(`${BACKEND_URL}/api/public/posts/${freePost.id}/comments`);
    const ids = (await list.json()).map((c: any) => c.id);
    expect(ids).not.toContain(own);
    expect(ids).not.toContain(foreign);
  });

  test('likes: entitlement, idempotency, and state', async () => {
    // Anonymous cannot like
    expect((await anon.post(`${BACKEND_URL}/api/posts/${freePost.id}/like`)).status()).toBe(401);
    // Non-entitled fan cannot like a gated post
    expect((await plainFan.post(`${BACKEND_URL}/api/posts/${gatedPost.id}/like`)).status()).toBe(403);

    // Entitled fan likes the gated post
    const like1 = await entitled.post(`${BACKEND_URL}/api/posts/${gatedPost.id}/like`);
    expect(like1.status()).toBe(200);
    const state1 = await like1.json();
    expect(state1.likedByMe).toBe(true);
    expect(state1.likeCount).toBe(1);

    // Liking again is idempotent
    const like2 = await entitled.post(`${BACKEND_URL}/api/posts/${gatedPost.id}/like`);
    expect(like2.status()).toBe(200);
    expect((await like2.json()).likeCount).toBe(1);

    // Like state via the public endpoint
    const mine = await entitled.get(`${BACKEND_URL}/api/public/posts/${gatedPost.id}/likes`);
    expect(mine.status()).toBe(200);
    expect(await mine.json()).toEqual({ likeCount: 1, likedByMe: true });
    const theirs = await entitled2.get(`${BACKEND_URL}/api/public/posts/${gatedPost.id}/likes`);
    expect((await theirs.json()).likedByMe).toBe(false);

    // Counts surface on the public feed
    const feed = await entitled.get(`${BACKEND_URL}/api/public/posts?limit=100`);
    const feedPost = (await feed.json()).find((p: any) => p.id === gatedPost.id);
    expect(feedPost.likeCount).toBe(1);
    expect(feedPost.likedByMe).toBe(true);
    expect(typeof feedPost.commentCount).toBe('number');

    // Unlike, idempotently
    const unlike1 = await entitled.delete(`${BACKEND_URL}/api/posts/${gatedPost.id}/like`);
    expect(unlike1.status()).toBe(200);
    expect(await unlike1.json()).toEqual({ likeCount: 0, likedByMe: false });
    const unlike2 = await entitled.delete(`${BACKEND_URL}/api/posts/${gatedPost.id}/like`);
    expect(unlike2.status()).toBe(200);
    expect((await unlike2.json()).likeCount).toBe(0);
  });

  test('comment count surfaces on the public feed', async () => {
    const res = await entitled.post(`${BACKEND_URL}/api/posts/${freePost.id}/comments`, {
      data: { content: 'count me' },
    });
    expect(res.status()).toBe(201);
    const feed = await anon.get(`${BACKEND_URL}/api/public/posts?limit=100`);
    const feedPost = (await feed.json()).find((p: any) => p.id === freePost.id);
    expect(feedPost.commentCount).toBeGreaterThanOrEqual(1);
  });

  test('UI: like button and comments on an unlocked post page', async ({ page }) => {
    // A dedicated post so counts start at zero
    const uiPost = await createPost(creatorCtx, await getFeedSeriesId(), 'eng-ui', {
      content: '<p>ui engagement post</p>',
    });

    await loginViaApi(page, plainFanUser.email, plainFanUser.password);
    await page.goto(`/posts/${uiPost.slug}`);

    // Like toggles the count
    const likeButton = page.getByTestId('like-button');
    const likeCount = page.getByTestId('like-count');
    await expect(likeCount).toHaveText('0');
    await likeButton.click();
    await expect(likeCount).toHaveText('1');
    await likeButton.click();
    await expect(likeCount).toHaveText('0');

    // Comment via the input
    await expect(page.getByTestId('comments-section')).toBeVisible();
    await page.getByTestId('comment-input').fill('A comment from the UI');
    await page.getByRole('button', { name: 'Post comment' }).click();
    await expect(page.getByTestId('comment')).toHaveCount(1);
    await expect(page.getByTestId('comment')).toContainText('A comment from the UI');
    await expect(page.getByText('Comments (1)')).toBeVisible();
  });
});
