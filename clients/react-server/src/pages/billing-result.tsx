import { JSX, useEffect, useState } from 'react';
import { Link } from 'react-router';
import PxBorder from '@/components/px-border';
import PublicHeader from '@/components/public-header';
import { getBillingMe } from '@/lib/api';

interface BillingResultProps {
  /**
   * Which checkout outcome this page represents
   */
  variant: 'success' | 'cancel';
}

/**
 * Post-checkout landing page for /billing/success and /billing/cancel. On
 * success it polls the billing state a few times while Stripe's webhook is
 * processed.
 *
 * @param {BillingResultProps} props - The component props
 * @param {'success' | 'cancel'} props.variant - Which outcome to show
 * @returns {JSX.Element} The BillingResult component
 */
export const BillingResult = ({ variant }: BillingResultProps): JSX.Element => {
  const [confirmed, setConfirmed] = useState(false);
  const [doneWaiting, setDoneWaiting] = useState(false);

  useEffect(() => {
    if (variant !== 'success') {
      return;
    }
    let cancelled = false;
    let attempts = 0;

    /**
     * Polls the billing state until a subscription or purchase shows up.
     */
    const poll = async (): Promise<void> => {
      attempts += 1;
      try {
        const billing = await getBillingMe();
        if (cancelled) return;
        if (billing.subscription || billing.purchases.length > 0) {
          setConfirmed(true);
          setDoneWaiting(true);
          return;
        }
      } catch {
        // Anonymous or transient error; keep polling
      }
      if (attempts < 6 && !cancelled) {
        setTimeout(() => void poll(), 2000);
      } else if (!cancelled) {
        setDoneWaiting(true);
      }
    };
    void poll();

    return () => {
      cancelled = true;
    };
  }, [variant]);

  return (
    <div className="min-h-screen">
      <PublicHeader />
      <main className="p-[50px] px-6 md:px-[100px]">
        <div className="relative mx-auto max-w-[600px] bg-white p-10 text-center">
          <PxBorder width={3} radius="lg" />
          {variant === 'cancel' ? (
            <>
              <h1 className="mb-4 text-3xl">Checkout cancelled</h1>
              <p className="mb-6 text-lg">No payment was made. You can try again any time.</p>
            </>
          ) : confirmed ? (
            <>
              <h1 className="mb-4 text-3xl">Payment confirmed</h1>
              <p className="mb-6 text-lg">Thank you for your support! Your access is active.</p>
            </>
          ) : (
            <>
              <h1 className="mb-4 text-3xl">Payment processing</h1>
              <p className="mb-6 text-lg">
                {doneWaiting
                  ? 'Your payment is still being processed. It can take a few seconds - refresh this page shortly.'
                  : 'Payment processing - this can take a few seconds...'}
              </p>
            </>
          )}
          <Link to="/" className="text-lg underline">
            Back to the feed
          </Link>
        </div>
      </main>
    </div>
  );
};

export default BillingResult;
