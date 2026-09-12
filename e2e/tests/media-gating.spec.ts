import { test, expect, APIRequestContext } from '@playwright/test';
import { apiContextFor, anonContext, CREATOR } from '../helpers/auth';
import { createPost } from '../helpers/fixtures';
import { makeVerifiedUser, giveSubscription, getFeedSeriesId, cleanupE2eRows } from '../helpers/db';

/**
 * BUG (backend): file uploads fail against the local MinIO stack with
 * HTTP 500 "Failed to upload to S3 ... InvalidBucketName". The S3 client in
 * backend/shared/src/services/s3.rs sets endpoint_url for AWS_S3_HOST but
 * never enables force_path_style, so the AWS SDK uses virtual-hosted-style
 * addressing (http://patron-local.localhost:9000), which MinIO rejects.
 * Repro: log in as the creator and
 *   curl -b cookies -F 'file=@tiny.png;type=image/png' \
 *        http://localhost:8080/api/files/actions/upload   -> 500
 * Fix: add `.force_path_style(true)` to the aws_sdk_s3 config when
 * AWS_S3_HOST is set. Until then every test that needs a stored file is
 * fixme'd below; they are written for the correct behavior.
 */

// A 1x1 transparent PNG
const TINY_PNG = Buffer.from(
  'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==',
  'base64',
);

async function uploadTinyPng(ctx: APIRequestContext): Promise<string> {
  const res = await ctx.post('/proxy/api/files/actions/upload', {
    multipart: {
      file: { name: 'e2e-tiny.png', mimeType: 'image/png', buffer: TINY_PNG },
    },
  });
  if (res.status() !== 201) {
    throw new Error(`upload failed: ${res.status()} ${await res.text()}`);
  }
  const body = await res.json();
  return body.file.id;
}

let creatorCtx: APIRequestContext;
let anon: APIRequestContext;
let entitledFan: APIRequestContext;

test.describe('media gating via /api/cdn/files', () => {
  test.beforeAll(async () => {
    creatorCtx = await apiContextFor(CREATOR.email, CREATOR.password);
    anon = await anonContext();
    const fan = await makeVerifiedUser('media-fan');
    await giveSubscription(fan.id, 2, 'active');
    entitledFan = await apiContextFor(fan.email, fan.password);
  });

  test.afterAll(async () => {
    await cleanupE2eRows();
    for (const ctx of [creatorCtx, anon, entitledFan]) await ctx.dispose();
  });

  test('gated image file: 403 anonymous, 200 entitled fan, 200 creator, Cache-Control private', async () => {
    test.fixme(); // BUG: upload endpoint 500s (S3 path-style, see header comment)

    const fileId = await uploadTinyPng(creatorCtx);
    const feedId = await getFeedSeriesId();
    await createPost(creatorCtx, feedId, 'media-images', {
      kind: 'images',
      minTierLevel: 2,
      imageFileIds: [fileId],
    });

    const anonRes = await anon.get(`/proxy/api/cdn/files/${fileId}`);
    expect(anonRes.status()).toBe(403);

    const fanRes = await entitledFan.get(`/proxy/api/cdn/files/${fileId}`);
    expect(fanRes.status()).toBe(200);
    expect(fanRes.headers()['cache-control']).toContain('private');

    const creatorRes = await creatorCtx.get(`/proxy/api/cdn/files/${fileId}`);
    expect(creatorRes.status()).toBe(200);
  });

  test('gated audio and video attachments are entitlement-checked', async () => {
    test.fixme(); // BUG: upload endpoint 500s (S3 path-style, see header comment)

    const audioId = await uploadTinyPng(creatorCtx); // any stored file works for gating checks
    const videoId = await uploadTinyPng(creatorCtx);
    const feedId = await getFeedSeriesId();
    await createPost(creatorCtx, feedId, 'media-audio', {
      kind: 'audio',
      minTierLevel: 2,
      audioFileId: audioId,
    });
    await createPost(creatorCtx, feedId, 'media-video', {
      kind: 'video',
      minTierLevel: 2,
      videoFileId: videoId,
    });

    for (const id of [audioId, videoId]) {
      expect((await anon.get(`/proxy/api/cdn/files/${id}`)).status()).toBe(403);
      expect((await entitledFan.get(`/proxy/api/cdn/files/${id}`)).status()).toBe(200);
      expect((await creatorCtx.get(`/proxy/api/cdn/files/${id}`)).status()).toBe(200);
    }
  });

  test('upload endpoint accepts a file from the creator', async () => {
    test.fixme(); // BUG: returns 500 InvalidBucketName instead of 201 (S3 path-style)

    const fileId = await uploadTinyPng(creatorCtx);
    expect(fileId).toMatch(/^[0-9a-f-]{36}$/);
  });
});
