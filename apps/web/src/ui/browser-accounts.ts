export interface BrowserAccount {
  id: string;
  displayName: string;
  email: string;
  kind: "demo" | "browser";
  createdAt: string;
}

export const BROWSER_ACCOUNTS_STORAGE_KEY = "evil.browser.accounts.v1";

export const DEFAULT_DEMO_ACCOUNT: BrowserAccount = {
  id: "demo-hunter-lab",
  displayName: "Hunter Lab Demo",
  email: "demo@ashen-frontier.local",
  kind: "demo",
  createdAt: "2026-07-29T00:00:00.000Z",
};

export function parseBrowserAccounts(raw: string | null): BrowserAccount[] {
  if (!raw) return [DEFAULT_DEMO_ACCOUNT];
  try {
    const value: unknown = JSON.parse(raw);
    if (!Array.isArray(value)) return [DEFAULT_DEMO_ACCOUNT];
    const accounts = value.filter(isBrowserAccount);
    return accounts.length ? accounts : [DEFAULT_DEMO_ACCOUNT];
  } catch {
    return [DEFAULT_DEMO_ACCOUNT];
  }
}

export function serializeBrowserAccounts(accounts: BrowserAccount[]): string {
  return JSON.stringify(accounts);
}

function isBrowserAccount(value: unknown): value is BrowserAccount {
  if (!value || typeof value !== "object") return false;
  const account = value as Partial<BrowserAccount>;
  return typeof account.id === "string"
    && typeof account.displayName === "string"
    && typeof account.email === "string"
    && (account.kind === "demo" || account.kind === "browser")
    && typeof account.createdAt === "string";
}
