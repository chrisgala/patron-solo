import { JSX, useEffect, useState } from 'react';
import { useNavigate } from 'react-router';
import PxBorder from '@/components/px-border';
import PublicHeader from '@/components/public-header';
import { Button } from '@/components/ui/button';
import { formatPrice } from '@/components/paywall-ctas';
import { useAuth } from '@/contexts/AuthContext';
import {
  BillingMeResponse,
  getBillingMe,
  getPublicTiers,
  portal,
  subscribe,
  TierResponse,
} from '@/lib/api';

/**
 * Membership page: lists the joinable tiers and the visitor's current
 * subscription state.
 *
 * @returns {JSX.Element} The Membership component
 */
export const Membership = (): JSX.Element => {
  const { user } = useAuth();
  const navigate = useNavigate();
  const [tiers, setTiers] = useState<TierResponse[] | null>(null);
  const [billing, setBilling] = useState<BillingMeResponse | null>(null);
  const [busyTierId, setBusyTierId] = useState<string | null>(null);
  const [portalBusy, setPortalBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    /**
     * Loads tiers and, for signed-in users, the billing state.
     */
    const load = async (): Promise<void> => {
      try {
        const tierList = await getPublicTiers();
        // eslint-disable-next-line max-params
        setTiers([...tierList].sort((a, b) => a.level - b.level));
      } catch (loadError) {
        console.error('Failed to load tiers:', loadError);
        setError('Failed to load membership tiers.');
        return;
      }
      if (user) {
        try {
          setBilling(await getBillingMe());
        } catch (billingError) {
          console.warn('Failed to load billing state:', billingError);
        }
      }
    };
    void load();
  }, [user]);

  /**
   * Starts a Stripe checkout for the given tier, or sends anonymous
   * visitors to registration.
   *
   * @param {string} tierId - The tier to join
   */
  const handleJoin = async (tierId: string): Promise<void> => {
    if (!user) {
      navigate('/register', { viewTransition: true });
      return;
    }
    setBusyTierId(tierId);
    try {
      const { url } = await subscribe(tierId);
      window.location.href = url;
    } catch (joinError) {
      console.error('Failed to start subscription checkout:', joinError);
      setBusyTierId(null);
    }
  };

  /**
   * Opens the Stripe billing portal.
   */
  const handlePortal = async (): Promise<void> => {
    setPortalBusy(true);
    try {
      const { url } = await portal();
      window.location.href = url;
    } catch (portalError) {
      console.error('Failed to open billing portal:', portalError);
      setPortalBusy(false);
    }
  };

  const subscription = billing?.subscription ?? null;

  return (
    <div className="min-h-screen">
      <PublicHeader />
      <main className="p-[50px] px-6 md:px-[100px]">
        <h1 className="mb-10 text-5xl">Membership</h1>
        {subscription && (
          <div className="relative mb-10 max-w-[600px] bg-white p-6">
            <PxBorder width={3} radius="lg" />
            <p className="mb-4 text-lg">
              You are subscribed at tier level {subscription.tierLevel}
              {subscription.cancelAtPeriodEnd && ' (cancels at period end)'}
              {subscription.currentPeriodEnd &&
                ` - renews ${new Date(subscription.currentPeriodEnd).toLocaleDateString()}`}
            </p>
            <Button
              variant="secondary"
              containerClassName="w-max"
              disabled={portalBusy}
              onClick={() => void handlePortal()}
            >
              {portalBusy ? 'Opening...' : 'Manage in billing portal'}
            </Button>
          </div>
        )}
        {error && <p className="text-lg">{error}</p>}
        {!error && tiers === null && <p className="text-lg">Loading tiers...</p>}
        {!error && tiers !== null && tiers.length === 0 && (
          <p className="text-lg">No membership tiers are available yet.</p>
        )}
        {!error && tiers !== null && tiers.length > 0 && (
          <div className="grid grid-cols-1 gap-10 sm:grid-cols-2 xl:grid-cols-3">
            {tiers.map((tier) => {
              const isCurrent = subscription?.tierId === tier.id;
              return (
                <div
                  key={tier.id}
                  className="bg-secondary-primary relative flex h-full flex-col justify-between gap-4 p-5"
                >
                  <PxBorder width={3} radius="lg" />
                  <div className="flex flex-col gap-5">
                    <div className="flex justify-between">
                      <h3 className="text-2xl">{tier.name}</h3>
                      <div className="flex flex-col items-end gap-[5px]">
                        <p className="text-4xl font-bold">{formatPrice(tier.priceCents)}</p>
                        <p className="text-base">per month</p>
                      </div>
                    </div>
                    {tier.description && (
                      <ul className="flex [list-style-type:square] flex-col gap-2 pl-5">
                        {tier.description
                          .split('\n')
                          .filter((feature) => feature.trim() !== '')
                          .map((feature) => (
                            <li key={feature} className="text-sm">
                              {feature}
                            </li>
                          ))}
                      </ul>
                    )}
                  </div>
                  {isCurrent ? (
                    <div className="relative m-[3px] bg-white px-3 py-2 text-center">
                      <PxBorder width={3} radius="md" />
                      <span className="text-base">Current plan</span>
                    </div>
                  ) : (
                    <Button
                      className="w-full"
                      containerClassName="mt-0"
                      disabled={busyTierId !== null}
                      onClick={() => void handleJoin(tier.id)}
                    >
                      {busyTierId === tier.id ? 'Redirecting...' : 'Join'}
                    </Button>
                  )}
                </div>
              );
            })}
          </div>
        )}
      </main>
    </div>
  );
};

export default Membership;
