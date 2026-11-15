import { ref } from "vue";

export const DEFAULT_PINNED_BRANCHES = ["develop", "main", "master"];

const version = ref(0);
const cache = new Map<string, string[] | null>();

function storageKey(path: string) {
  return `shipyard:pinned-branches:${path}`;
}

function readStored(path: string): string[] | null {
  try {
    const raw = localStorage.getItem(storageKey(path));
    if (raw === null) {
      return null;
    }
    const parsed = JSON.parse(raw);
    return Array.isArray(parsed) ? parsed.filter((item) => typeof item === "string") : null;
  } catch {
    return null;
  }
}

/**
 * Pins live per repo. Until the user changes them, a repo uses the default
 * pins; after that the stored list wins, so unpinning a default sticks.
 */
export function pinnedBranches(path: string): string[] {
  void version.value;
  if (!cache.has(path)) {
    cache.set(path, readStored(path));
  }
  return cache.get(path) ?? DEFAULT_PINNED_BRANCHES;
}

export function setBranchPinned(path: string, name: string, pinned: boolean) {
  const next = pinnedBranches(path).filter((item) => item !== name);
  if (pinned) {
    next.push(name);
  }
  cache.set(path, next);
  version.value += 1;
  try {
    localStorage.setItem(storageKey(path), JSON.stringify(next));
  } catch {
    /* pins just won't persist */
  }
}

export function pinnedFirst<T>(items: T[], pinned: string[], name: (item: T) => string): T[] {
  const set = new Set(pinned);
  return [
    ...items.filter((item) => set.has(name(item))),
    ...items.filter((item) => !set.has(name(item))),
  ];
}
