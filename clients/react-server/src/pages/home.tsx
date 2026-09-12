import { JSX, useEffect, useState } from 'react';
import { Link } from 'react-router';
import { Lock } from 'lucide-react';
import PxBorder from '@/components/px-border';
import FocusRing from '@/components/focus-ring';
import PublicHeader from '@/components/public-header';
import PaywallCtas from '@/components/paywall-ctas';
import { getPublicPosts, getSite, CreatorProfile, PublicPostResponse } from '@/lib/api';

/**
 * Strips HTML tags from rich text content and returns a short excerpt.
 *
 * @param {string} html - The HTML content
 * @returns {string} A plain-text excerpt
 */
const excerpt = (html: string): string => {
  const text = html
    .replace(/<[^>]*>/g, ' ')
    .replace(/\s+/g, ' ')
    .trim();
  return text.length > 220 ? `${text.slice(0, 220)}...` : text;
};

interface PostCardProps {
  /**
   * The post to render
   */
  post: PublicPostResponse;
}

/**
 * A single post card in the public feed.
 *
 * @param {PostCardProps} props - The component props
 * @param {PublicPostResponse} props.post - The post to render
 * @returns {JSX.Element} The post card
 */
const PostCard = ({ post }: PostCardProps): JSX.Element => {
  const locked = !post.access.granted;

  return (
    <article className="bg-secondary-primary relative flex h-full flex-col gap-4 p-5">
      <PxBorder width={3} radius="lg" />
      <div className="bg-accent relative aspect-video">
        <PxBorder width={3} radius="lg" />
        {post.thumbnailUrl ? (
          <img src={post.thumbnailUrl} alt={post.title} className="h-full w-full object-cover" />
        ) : (
          <div className="flex h-full w-full items-center justify-center">
            <img src="/assets/series.png" alt="post" className="h-full w-full object-cover" />
          </div>
        )}
        {locked && (
          <div className="absolute inset-0 z-10 flex items-center justify-center bg-black/50">
            <Lock size={40} color="white" />
          </div>
        )}
        <div className="absolute right-2.5 bottom-2.5 z-10">
          <div className="relative m-[3px] bg-white px-1.5 py-[3px]">
            <PxBorder width={3} radius="md" />
            <span className="text-sm capitalize">{post.kind}</span>
          </div>
        </div>
      </div>
      <div className="flex flex-1 flex-col gap-2">
        {locked ? (
          <h3 className="text-xl">{post.title}</h3>
        ) : (
          <Link className="group relative w-max outline-none" to={`/posts/${post.slug}`}>
            <FocusRing width={3} />
            <h3 className="text-xl group-hover:underline">{post.title}</h3>
          </Link>
        )}
        {post.createdAt && (
          <p className="text-sm">{new Date(post.createdAt).toLocaleDateString()}</p>
        )}
        {!locked && (post.kind === 'article' || post.kind === 'update') && post.content && (
          <p className="text-base">{excerpt(post.content)}</p>
        )}
        {!locked && post.kind !== 'update' && (
          <Link to={`/posts/${post.slug}`} className="text-base underline">
            {post.kind === 'article' ? 'Read more' : 'View post'}
          </Link>
        )}
      </div>
      {locked && <PaywallCtas access={post.access} postId={post.id} />}
    </article>
  );
};

/**
 * Public home page: creator profile header and the post feed.
 *
 * @returns {JSX.Element} The Home component
 */
export const Home = (): JSX.Element => {
  const [creator, setCreator] = useState<CreatorProfile | null>(null);
  const [posts, setPosts] = useState<PublicPostResponse[] | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    /**
     * Loads the site profile and public feed.
     */
    const load = async (): Promise<void> => {
      try {
        const [site, feed] = await Promise.all([getSite(), getPublicPosts({ limit: 30 })]);
        setCreator(site.creator);
        setPosts(feed);
      } catch (loadError) {
        console.error('Failed to load public feed:', loadError);
        setError('Failed to load the feed. Please try again later.');
      }
    };
    void load();
  }, []);

  return (
    <div className="min-h-screen">
      <PublicHeader creator={creator} />
      <main className="p-[50px] px-6 md:px-[100px]">
        {error && (
          <div className="relative mx-auto max-w-[600px] bg-white p-10 text-center">
            <PxBorder width={3} radius="lg" />
            <p className="text-lg">{error}</p>
          </div>
        )}
        {!error && posts === null && <p className="text-center text-lg">Loading posts...</p>}
        {!error && posts !== null && posts.length === 0 && (
          <div className="relative mx-auto max-w-[600px] bg-white p-10 text-center">
            <PxBorder width={3} radius="lg" />
            <p className="text-lg">No posts yet. Check back soon!</p>
          </div>
        )}
        {!error && posts !== null && posts.length > 0 && (
          <div className="grid grid-cols-1 gap-10 sm:grid-cols-2 xl:grid-cols-3">
            {posts.map((post) => (
              <PostCard key={post.id} post={post} />
            ))}
          </div>
        )}
      </main>
    </div>
  );
};

export default Home;
