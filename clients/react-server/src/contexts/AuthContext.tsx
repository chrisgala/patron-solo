import { createContext, useContext, useState, useEffect, ReactNode, JSX } from 'react';
import { UserInfo } from 'patronts/models';

const API_BASE: string = import.meta.env.VITE_SERVER_URL || 'http://localhost:8080';

interface AuthContextType {
  user: UserInfo | null;
  // eslint-disable-next-line no-unused-vars
  setUser: (user: UserInfo | null) => void;
  isCreator: boolean;
  /** True while the role is still being recovered for a logged-in user */
  isRoleLoading: boolean;
}

const AuthContext = createContext<AuthContextType | undefined>(undefined);

interface AuthProviderProps {
  children: ReactNode;
  initialUser?: UserInfo | null;
}

/**
 * Authentication provider component that manages auth state and provides auth methods.
 *
 * @param {AuthProviderProps} props - The component props
 * @param {ReactNode} props.children - Child components to render
 * @param {UserInfo | null} props.initialUser - Initial user data from server
 * @returns {JSX.Element} The provider component
 */
export const AuthProvider = ({ children, initialUser }: AuthProviderProps): JSX.Element => {
  const [user, setUser] = useState<UserInfo | null>(initialUser ?? null);
  const [role, setRole] = useState<string | null>(
    (initialUser as (UserInfo & { role?: string }) | null)?.role ?? null,
  );

  // The published patronts SDK strips the new `role` field during zod
  // parsing, so fetch it from the raw endpoint whenever it's missing.
  useEffect(() => {
    const inlineRole = (user as (UserInfo & { role?: string }) | null)?.role;
    if (!user) {
      setRole(null);
      return;
    }
    if (inlineRole) {
      setRole(inlineRole);
      return;
    }
    /**
     * Fetches the raw /api/auth/me response to recover the role field.
     */
    const loadRole = async (): Promise<void> => {
      try {
        const response = await fetch(`${API_BASE}/api/auth/me`, { credentials: 'include' });
        if (response.ok) {
          const raw = (await response.json()) as { role?: string };
          setRole(raw.role ?? 'fan');
        }
      } catch {
        // Keep role unknown; UI treats it as fan
      }
    };
    void loadRole();
  }, [user]);

  const isCreator = role === 'creator';
  const isRoleLoading = user !== null && role === null;

  const value = {
    user,
    setUser,
    isCreator,
    isRoleLoading,
  };

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>;
};

/**
 * Custom hook to access authentication context.
 *
 * @returns {AuthContextType} The authentication context
 * @throws {Error} When used outside of AuthProvider
 */
export const useAuth = (): AuthContextType => {
  const context = useContext(AuthContext);
  if (context === undefined) {
    throw new Error('useAuth must be used within an AuthProvider');
  }
  return context;
};
