import { JSX, useEffect, useState } from 'react';
import { Link } from 'react-router';
import { Bell, BellOff } from 'lucide-react';
import PxBorder from '@/components/px-border';
import { Button } from '@/components/ui/button';
import { useAuth } from '@/contexts/AuthContext';
import { CreatorProfile } from '@/lib/api';
import { disablePush, enablePush, isPushEnabled, isPushSupported } from '@/lib/push';

interface PublicHeaderProps {
  /**
   * Creator profile to show in the banner (omit for a nav-only header)
   */
  creator?: CreatorProfile | null;
}

/**
 * Bell button that toggles web-push notifications for new posts. Hidden when
 * push is unsupported by the browser or not configured on the server.
 *
 * @returns {JSX.Element | null} The bell button or null
 */
const PushBell = (): JSX.Element | null => {
  const [supported, setSupported] = useState(false);
  const [enabled, setEnabled] = useState(false);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    /**
     * Detects push support and current subscription state.
     */
    const detect = async (): Promise<void> => {
      const ok = await isPushSupported();
      setSupported(ok);
      if (ok) {
        setEnabled(await isPushEnabled());
      }
    };
    void detect().catch(() => setSupported(false));
  }, []);

  if (!supported) {
    return null;
  }

  /**
   * Toggles the push subscription.
   */
  const handleToggle = async (): Promise<void> => {
    setBusy(true);
    try {
      if (enabled) {
        await disablePush();
        setEnabled(false);
      } else {
        setEnabled(await enablePush());
      }
    } catch (error) {
      console.error('Failed to toggle push notifications:', error);
    } finally {
      setBusy(false);
    }
  };

  return (
    <Button
      variant="secondary"
      shadow={false}
      containerClassName="w-max"
      disabled={busy}
      onClick={() => void handleToggle()}
      title={enabled ? 'Disable new post notifications' : 'Notify me about new posts'}
    >
      {enabled ? <BellOff size={18} /> : <Bell size={18} />}
      {enabled ? 'Notifications on' : 'Notify me about new posts'}
    </Button>
  );
};

/**
 * Header for the public pages: top navigation plus an optional creator
 * banner with avatar, display name and bio.
 *
 * @param {PublicHeaderProps} props - The component props
 * @param {CreatorProfile | null} props.creator - Creator profile for the banner
 * @returns {JSX.Element} The public header
 */
const PublicHeader = ({ creator }: PublicHeaderProps): JSX.Element => {
  const { user, isCreator } = useAuth();

  return (
    <header>
      <nav className="flex items-center justify-between gap-4 border-b-5 border-b-black bg-white px-6 py-3">
        <div className="flex items-center gap-6">
          <Link to="/" className="flex items-center gap-2.5">
            <img src="/logo.svg" alt="logo" className="size-8" />
          </Link>
          <Link to="/" className="text-lg hover:underline">
            Home
          </Link>
          <Link to="/membership" className="text-lg hover:underline">
            Membership
          </Link>
          {user && (
            <Link to="/settings" className="text-lg hover:underline">
              Settings
            </Link>
          )}
          {isCreator && (
            <Link to="/dashboard/content" className="text-lg hover:underline">
              Dashboard
            </Link>
          )}
        </div>
        <div className="flex items-center gap-4">
          <PushBell />
          {!user && (
            <Link to="/login" className="text-lg underline">
              Log in
            </Link>
          )}
        </div>
      </nav>
      {creator && (
        <div
          className="bg-secondary-primary relative flex w-full items-center gap-[25px] border-b-5 border-b-black p-[50px] px-[100px]"
          style={
            creator.banner
              ? {
                  backgroundImage: `url(${creator.banner})`,
                  backgroundSize: 'cover',
                  backgroundPosition: 'center',
                }
              : undefined
          }
        >
          {creator.avatarUrl && (
            <div className="relative m-[5px] size-[140px] shrink-0">
              <img src={creator.avatarUrl} alt="pfp" className="size-full object-cover" />
              <PxBorder width={5} radius="lg" />
            </div>
          )}
          <div className="flex flex-col gap-5">
            {creator.displayName && (
              <div className="relative m-[5px] w-max">
                <PxBorder width={5} radius="md" />
                <div className="bg-white px-[10px] py-[5px]">
                  <h1 className="text-3xl">{creator.displayName}</h1>
                </div>
              </div>
            )}
            {creator.description && (
              <div className="relative m-[5px]">
                <PxBorder width={5} radius="md" />
                <div className="bg-white px-[10px] py-[5px]">
                  <p className="text-lg">{creator.description}</p>
                </div>
              </div>
            )}
          </div>
        </div>
      )}
    </header>
  );
};

export default PublicHeader;
