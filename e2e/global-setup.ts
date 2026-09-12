import { FRONTEND_URL, BACKEND_URL } from './playwright.config';

const STACK_HELP = `
The local stack must be running before the e2e suite (see DEVELOPMENT.md):

  docker compose up -d postgres redis mailpit minio minio-init
  cd backend && cargo run --bin server          # http://localhost:8080
  cd clients/react-server && npm run dev:local  # http://localhost:5173
`;

/** Fails fast with a clear message when part of the stack is not reachable. */
async function check(name: string, url: string): Promise<void> {
  try {
    const res = await fetch(url);
    if (!res.ok) throw new Error(`HTTP ${res.status}`);
  } catch (err) {
    throw new Error(`${name} is not reachable at ${url} (${err}).\n${STACK_HELP}`);
  }
}

export default async function globalSetup(): Promise<void> {
  await check('Frontend (SSR dev server)', FRONTEND_URL);
  await check('Backend API', `${BACKEND_URL}/api/public/site`);
  await check('Mailpit', 'http://localhost:8025/api/v1/messages');
}
