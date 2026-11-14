import { computed, ref, watch } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { ResolvedTheme, ThemePreference } from "./types";

export const THEME_OPTIONS: { id: ThemePreference; label: string }[] = [
  { id: "system", label: "System" },
  { id: "light", label: "Light" },
  { id: "dark", label: "Dark" },
];

/** The last saved theme, so the window opens in it before the settings load. */
const STORAGE_KEY = "theme";

/**
 * CodeMirror theme rules for the parts its base theme colors for a dark
 * editor, so they follow the app theme instead. Selectors match the base
 * theme's so these win.
 */
export const editorChrome = {
  ".cm-selectionBackground, &.cm-focused > .cm-scroller > .cm-selectionLayer .cm-selectionBackground":
    { backgroundColor: "var(--selection)" },
  ".cm-panels": { backgroundColor: "var(--bg-raised)", color: "var(--text)" },
  ".cm-panels-top": { borderBottom: "1px solid var(--border)" },
  ".cm-panels-bottom": { borderTop: "1px solid var(--border)" },
  ".cm-textfield": {
    backgroundColor: "var(--bg-input)",
    border: "1px solid var(--border)",
    color: "var(--text)",
  },
  ".cm-button": {
    backgroundImage: "none",
    backgroundColor: "var(--bg-raised)",
    border: "1px solid var(--border)",
    color: "var(--text)",
  },
  ".cm-searchMatch": { backgroundColor: "var(--search-match)" },
  ".cm-searchMatch-selected": { backgroundColor: "var(--search-match-selected)" },
};

export function sanitizeTheme(value: unknown): ThemePreference {
  return value === "light" || value === "dark" ? value : "system";
}

function readStoredTheme() {
  try {
    return sanitizeTheme(localStorage.getItem(STORAGE_KEY));
  } catch {
    return "system";
  }
}

const systemQuery = window.matchMedia("(prefers-color-scheme: light)");
const systemTheme = ref<ResolvedTheme>(systemQuery.matches ? "light" : "dark");
systemQuery.addEventListener("change", (event) => {
  systemTheme.value = event.matches ? "light" : "dark";
});

export const themePreference = ref<ThemePreference>(readStoredTheme());

export const resolvedTheme = computed<ResolvedTheme>(() =>
  themePreference.value === "system" ? systemTheme.value : themePreference.value,
);

watch(
  resolvedTheme,
  (theme) => {
    document.documentElement.dataset.theme = theme;
  },
  { immediate: true },
);

/**
 * The native window follows the choice too, so the title bar matches. With
 * System it goes back to following the OS, which also keeps the
 * prefers-color-scheme query reporting the OS setting.
 */
async function applyWindowTheme(preference: ThemePreference) {
  try {
    await getCurrentWindow().setTheme(preference === "system" ? null : preference);
  } catch {
    /* not every platform can theme the native window */
  }
}

watch(
  themePreference,
  (preference) => {
    try {
      localStorage.setItem(STORAGE_KEY, preference);
    } catch {
      /* the cached theme only avoids a flash on launch */
    }
    void applyWindowTheme(preference);
  },
  { immediate: true },
);
