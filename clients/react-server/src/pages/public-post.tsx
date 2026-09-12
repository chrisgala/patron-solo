import { JSX, useEffect, useState } from 'react';
import { Link, useLocation } from 'react-router';
import PxBorder from '@/components/px-border';
import PublicHeader from '@/components/public-header';
import PaywallCtas from '@/components/paywall-ctas';
import { useAuth } from '@/contexts/AuthContext';
import { cdnFileUrl, getPublicPost, PublicPostResponse } from '@/lib/api';

/**
 * Renders the media/body of an unlocked post according to its kind.
 *
 * @param {object} props - The component props
 * @param {PublicPostResponse} props.post - The unlocked post
 * @returns {JSX.Element} The post body
 */
const PostBody = ({ post }: { post: PublicPostResponse }): JSX.Element => {
  if (post.kind === 'audio' && post.audioFileId) {
    return (
      <div className="p-10">
        <audio controls src={cdnFileUrl(post.audioFileId)} className="w-full" />
      </div>
    );
  }

  if (post.kind === 'video' && post.videoFileId) {
    return (
      <div className="p-10">
        <video controls src={cdnFileUrl(post.videoFileId)} className="w-full" />
      </div>
    );
  }

  if (post.kind === 'images' && post.imageFileIds && post.imageFileIds.length > 0) {
    return (
      <div className="grid grid-cols-1 gap-5 p-10 sm:grid-cols-2">
        {post.imageFileIds.map((fileId) => (
          <div key={fileId} className="relative">
            <PxBorder width={3} radius="lg" />
            <img src={cdnFileUrl(fileId)} alt="" className="h-full w-full object-cover" />
          </div>
        ))}
      </div>
    );
  }

  // article / update: rich text HTML from the editor
  return (
    <div className="prose max-w-none p-10">
      <div
        className="prose-black prose-img:border-5 prose-img:rounded-[6px] prose-img:border-black prose-li:m-0 prose-h1:text-4xl prose-ul:color-black prose-strong:font-bold prose-p:text-base prose-img:m-0 prose-headings:m-0 prose-p:m-0 prose-ul:m-0 prose-ol:m-0 flex flex-col gap-5 leading-relaxed text-black"
        dangerouslySetInnerHTML={{ __html: post.content || '' }}
      />
    </div>
  );
};

/**
 * Public post page: renders the full post per kind, or a paywall panel when
 * the post is locked for the current visitor.
 *
 * @returns {JSX.Element} The PublicPost component
 */
export const PublicPost = (): JSX.Element => {
  const location = useLocation();
  const { user } = useAuth();
  const slug = location.pathname.replace(/^\/?posts\//, '').replace(/\/$/, '');
  const [post, setPost] = useState<PublicPostResponse | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    /**
     * Loads the post by slug.
     */
    const load = async (): Promise<void> => {
      if (!slug) {
        setError('Post not found');
        return;
      }
      try {
        setPost(await getPublicPost(slug));
      } catch (loadError) {
        console.error('Failed to load post:', loadError);
        setError('This post does not exist.');
      }
    };
    void load();
  }, [slug]);

  return (
    <div className="min-h-screen">
      <PublicHeader />
      <main className="p-[50px] px-6 md:px-[100px]">
        {error && (
          <div className="relative mx-auto max-w-[600px] bg-white p-10 text-center">
            <PxBorder width={3} radius="lg" />
            <p className="text-lg">{error}</p>
          </div>
        )}
        {!error && !post && <p className="text-center text-lg">Loading post...</p>}
        {!error && post && (
          <div className="bg-secondary-primary relative mx-auto w-full max-w-[1200px]">
            <PxBorder width={5} radius="lg" />
            <div className="flex flex-col gap-3 p-10">
              <h1 className="text-3xl font-bold">{post.title}</h1>
              <div className="flex items-center gap-3">
                <span className="text-sm capitalize">{post.kind}</span>
                {post.createdAt && (
                  <span className="text-sm">{new Date(post.createdAt).toLocaleDateString()}</span>
                )}
              </div>
            </div>
            <div className="border-t-5 border-black bg-white">
              {post.access.granted ? (
                <PostBody post={post} />
              ) : (
                <div className="flex flex-col items-center gap-6 p-10">
                  {post.thumbnailUrl && (
                    <div className="relative aspect-video w-full max-w-[600px]">
                      <PxBorder width={3} radius="lg" />
                      <img
                        src={post.thumbnailUrl}
                        alt={post.title}
                        className="h-full w-full object-cover"
                      />
                    </div>
                  )}
                  <PaywallCtas access={post.access} postId={post.id} />
                  {!user && (
                    <p className="text-base">
                      Already a member?{' '}
                      <Link to="/login" className="underline">
                        Log in
                      </Link>
                    </p>
                  )}
                </div>
              )}
            </div>
          </div>
        )}
      </main>
    </div>
  );
};

export default PublicPost;
