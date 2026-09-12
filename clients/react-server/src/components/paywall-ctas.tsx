import { JSX, useState } from 'react';
import { Link } from 'react-router';
import { Lock } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { PostAccess, purchase } from '@/lib/api';

/**
 * Formats a price in cents as a dollar string, e.g. 500 -> "$5.00".
 *
 * @param {number} cents - The price in cents
 * @returns {string} The formatted price
 */
export const formatPrice = (cents: number): string => `$${(cents / 100).toFixed(2)}`;

interface PaywallCtasProps {
  /**
   * Access metadata of the locked post
   */
  access: PostAccess;
  /**
   * Id of the post (used for one-off purchases)
   */
  postId: string;
}

/**
 * Call-to-action buttons for a locked post: join a tier, buy the post, or a
 * note about when it becomes free.
 *
 * @param {PaywallCtasProps} props - The component props
 * @param {PostAccess} props.access - Access metadata of the locked post
 * @param {string} props.postId - Id of the post
 * @returns {JSX.Element} The CTA block
 */
const PaywallCtas = ({ access, postId }: PaywallCtasProps): JSX.Element => {
  const [isBuying, setIsBuying] = useState(false);

  /**
   * Starts a Stripe checkout for a one-off purchase of this post.
   */
  const handleBuy = async (): Promise<void> => {
    setIsBuying(true);
    try {
      const { url } = await purchase({ postId });
      window.location.href = url;
    } catch (error) {
      console.error('Failed to start purchase:', error);
      setIsBuying(false);
    }
  };

  return (
    <div className="flex flex-col items-center gap-3">
      <div className="flex items-center gap-2">
        <Lock size={18} />
        <span className="text-base">This post is for members</span>
      </div>
      <div className="flex flex-wrap items-center justify-center gap-3">
        {access.requiredTierLevel !== null && (
          <Button asChild containerClassName="w-max">
            <Link to="/membership">Join tier {access.requiredTierLevel}</Link>
          </Button>
        )}
        {access.priceCents !== null && (
          <Button
            variant="secondary"
            containerClassName="w-max"
            disabled={isBuying}
            onClick={() => void handleBuy()}
          >
            {isBuying ? 'Redirecting...' : `Buy for ${formatPrice(access.priceCents)}`}
          </Button>
        )}
      </div>
      {access.freeAt && (
        <p className="text-sm">Free on {new Date(access.freeAt).toLocaleDateString()}</p>
      )}
    </div>
  );
};

export default PaywallCtas;
