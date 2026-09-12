import { JSX, StrictMode } from 'react';
import { hydrateRoot } from 'react-dom/client';
import { RouterProvider, createBrowserRouter } from 'react-router';
import { UserInfo } from 'patronts/models';
import { AuthProvider } from '@/contexts/AuthContext';
import { AppDataProvider } from '@/contexts/AppDataContext';
import Home from '@/pages/home';
import { Login } from '@/pages/login';
import { Register } from '@/pages/register';
import ProtectedRoute from '@/components/ProtectedRoute';
import { ForgotPasswordPage } from '@/pages/forgot-password';
import Content from '@/pages/dashboard/content';
import Insights from '@/pages/dashboard/insights';
import Audience from '@/pages/dashboard/audience';
import Payouts from '@/pages/dashboard/payouts';
import Settings from '@/pages/settings';
import ErrorBoundary from '@/components/ErrorBoundary';
import NewPost from '@/pages/new-post';
import EditPost from '@/pages/edit-post';
import Series from '@/pages/series';
import Post from '@/pages/post';
import PublicPost from '@/pages/public-post';
import Membership from '@/pages/membership';
import BillingResult from '@/pages/billing-result';
import { registerServiceWorker } from '@/lib/push';

const initialData = (window as any).__INITIAL_DATA__ as {
  user?: UserInfo | null;
  posts?: any[];
  series?: any[];
  singleSeries?: any;
  singlePost?: any;
} | null;

export const router = createBrowserRouter([
  {
    path: '/',
    errorElement: <ErrorBoundary />,
    children: [
      {
        index: true,
        Component: () => <Home />,
        errorElement: <ErrorBoundary />,
      },
      {
        path: 'posts/:slug',
        Component: () => <PublicPost />,
        errorElement: <ErrorBoundary />,
      },
      {
        path: 'membership',
        Component: () => <Membership />,
        errorElement: <ErrorBoundary />,
      },
      {
        path: 'billing/success',
        Component: () => <BillingResult variant="success" />,
        errorElement: <ErrorBoundary />,
      },
      {
        path: 'billing/cancel',
        Component: () => <BillingResult variant="cancel" />,
        errorElement: <ErrorBoundary />,
      },
      {
        path: 'login',
        Component: () => (
          <ProtectedRoute requireAuth={false}>
            <Login />
          </ProtectedRoute>
        ),
        errorElement: <ErrorBoundary />,
      },
      {
        path: 'register',
        Component: () => (
          <ProtectedRoute requireAuth={false}>
            <Register />
          </ProtectedRoute>
        ),
        errorElement: <ErrorBoundary />,
      },
      {
        path: 'reset-password',
        Component: () => (
          <ProtectedRoute requireAuth={false}>
            <ForgotPasswordPage />
          </ProtectedRoute>
        ),
        errorElement: <ErrorBoundary />,
      },
      {
        path: 'dashboard/content',
        Component: () => (
          <ProtectedRoute requireAuth={true} requireCreator={true}>
            <Content />
          </ProtectedRoute>
        ),
        errorElement: <ErrorBoundary />,
      },
      {
        path: 'dashboard/insights',
        Component: () => (
          <ProtectedRoute requireAuth={true} requireCreator={true}>
            <Insights />
          </ProtectedRoute>
        ),
        errorElement: <ErrorBoundary />,
      },
      {
        path: 'dashboard/audience',
        Component: () => (
          <ProtectedRoute requireAuth={true} requireCreator={true}>
            <Audience />
          </ProtectedRoute>
        ),
        errorElement: <ErrorBoundary />,
      },
      {
        path: 'dashboard/payouts',
        Component: () => (
          <ProtectedRoute requireAuth={true} requireCreator={true}>
            <Payouts />
          </ProtectedRoute>
        ),
        errorElement: <ErrorBoundary />,
      },
      {
        path: 'new-post',
        Component: () => (
          <ProtectedRoute requireAuth={true} requireCreator={true}>
            <NewPost />
          </ProtectedRoute>
        ),
        errorElement: <ErrorBoundary />,
      },
      {
        path: 'edit-post',
        Component: () => (
          <ProtectedRoute requireAuth={true} requireCreator={true}>
            <EditPost />
          </ProtectedRoute>
        ),
        errorElement: <ErrorBoundary />,
      },
      {
        path: 'settings',
        Component: () => (
          <ProtectedRoute requireAuth={true}>
            <Settings />
          </ProtectedRoute>
        ),
        errorElement: <ErrorBoundary />,
      },
      {
        path: 'series/:seriesId',
        Component: () => (
          <ProtectedRoute requireAuth={true} requireCreator={true}>
            <Series />
          </ProtectedRoute>
        ),
        errorElement: <ErrorBoundary />,
      },
      {
        path: 'post/:postId',
        Component: () => (
          <ProtectedRoute requireAuth={true} requireCreator={true}>
            <Post />
          </ProtectedRoute>
        ),
        errorElement: <ErrorBoundary />,
      },
    ],
  },
  {
    path: '*',
    Component: () => {
      throw new Response('Not Found', { status: 404 });
    },
    errorElement: <ErrorBoundary />,
  },
]);

/**
 * Root component that wraps the app with providers.
 *
 * @returns {JSX.Element} The app component with providers
 */
const App = (): JSX.Element => {
  return (
    <AuthProvider initialUser={initialData?.user}>
      <AppDataProvider
        initialPosts={initialData?.posts}
        initialSeries={initialData?.series}
        initialSingleSeries={initialData?.singleSeries}
        initialSinglePost={initialData?.singlePost}
      >
        <RouterProvider router={router} />
      </AppDataProvider>
    </AuthProvider>
  );
};

hydrateRoot(
  document.getElementById('root') as HTMLElement,
  <StrictMode>
    <App />
  </StrictMode>,
);

void registerServiceWorker();
