import { vi } from "./vi";

export const SUPPORTED_LOCALES = ["vi"] as const;
export type AppLocale = typeof SUPPORTED_LOCALES[number];
export type MessageKey = keyof typeof vi;
export type MessageParams = Readonly<Record<string, string | number>>;

const catalogs: Record<AppLocale, Record<MessageKey, string>> = { vi };
let activeLocale: AppLocale = "vi";

export function setLocale(locale: string): AppLocale {
  activeLocale = normalizeLocale(locale);
  if (typeof document !== "undefined") document.documentElement.lang = activeLocale;
  return activeLocale;
}

export function currentLocale(): AppLocale {
  return activeLocale;
}

export function t(key: MessageKey, params: MessageParams = {}): string {
  const template = catalogs[activeLocale][key] ?? catalogs.vi[key];
  return template.replace(/\{([a-zA-Z0-9_]+)\}/g, (match, name: string) => (
    Object.hasOwn(params, name) ? String(params[name]) : match
  ));
}

export function formatNumber(value: number): string {
  return new Intl.NumberFormat(activeLocale).format(value);
}

function normalizeLocale(locale: string): AppLocale {
  const language = locale.trim().replace("_", "-").split("-")[0].toLowerCase();
  return SUPPORTED_LOCALES.includes(language as AppLocale) ? language as AppLocale : "vi";
}

setLocale("vi");
