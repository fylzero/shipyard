export const LAST_FALLBACKS = ["develop", "master", "main"] as const;
export type LastFallback = (typeof LAST_FALLBACKS)[number];

export function isLastFallback(value: string): value is LastFallback {
  return (LAST_FALLBACKS as readonly string[]).includes(value);
}

export function splitFallbacks(list: string[]): { extra: string[]; last: LastFallback } {
  const cleaned = list.map((item) => item.trim()).filter(Boolean);
  const last = cleaned[cleaned.length - 1];
  if (last && isLastFallback(last)) {
    return { extra: cleaned.slice(0, -1), last };
  }
  return { extra: cleaned, last: "develop" };
}
