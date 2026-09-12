const MAILPIT = 'http://localhost:8025';

export interface MailpitSummary {
  ID: string;
  Subject: string;
  To: { Address: string }[];
  Created: string;
}

export async function listMessages(): Promise<MailpitSummary[]> {
  const res = await fetch(`${MAILPIT}/api/v1/messages?limit=50`);
  const body = (await res.json()) as { messages: MailpitSummary[] };
  return body.messages ?? [];
}

/** Polls Mailpit until a message addressed to `email` arrives. */
export async function waitForMessage(
  email: string,
  { subjectContains, timeoutMs = 15_000 }: { subjectContains?: string; timeoutMs?: number } = {},
): Promise<MailpitSummary> {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    const messages = await listMessages();
    const match = messages.find(
      (m) =>
        m.To.some((t) => t.Address.toLowerCase() === email.toLowerCase()) &&
        (!subjectContains || m.Subject.includes(subjectContains)),
    );
    if (match) return match;
    await new Promise((r) => setTimeout(r, 500));
  }
  throw new Error(`No Mailpit message for ${email} within ${timeoutMs}ms`);
}

export async function getMessageBody(id: string): Promise<string> {
  const res = await fetch(`${MAILPIT}/api/v1/message/${id}`);
  const body = (await res.json()) as { HTML?: string; Text?: string };
  return body.HTML || body.Text || '';
}

/** Extracts the email-verification link (backend verify-email URL) from a message. */
export async function extractVerificationLink(messageId: string): Promise<string> {
  const body = await getMessageBody(messageId);
  const match = body.match(/https?:\/\/[^"'<>\s]*verify-email\?token=[a-f0-9-]+/i);
  if (!match) throw new Error(`No verification link found in message ${messageId}`);
  return match[0];
}

export async function clearMessages(): Promise<void> {
  await fetch(`${MAILPIT}/api/v1/messages`, { method: 'DELETE' });
}
