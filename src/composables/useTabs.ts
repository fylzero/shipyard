import { computed, ref } from "vue";
import { useRouter } from "vue-router";
import { useApp } from "./useApp";

export const GROUPS_TAB_ID = "groups";
export const HISTORY_TAB_ID = "history";
export const SETTINGS_TAB_ID = "settings";
export const CHANGELOG_TAB_ID = "changelog";

export const SETTINGS_SECTIONS = ["general", "git", "schedule", "window", "json"] as const;
export type SettingsSection = (typeof SETTINGS_SECTIONS)[number];

type UtilityPanel = "settings" | "history" | "changelog";

export interface AppTab {
  id: string;
  title: string;
  closable: boolean;
  accentColor?: string;
}

interface RepoTab {
  id: string;
  title: string;
}

const repoTabs = ref<RepoTab[]>([]);
const historyTabOpen = ref(false);
const settingsTabOpen = ref(false);
const changelogTabOpen = ref(false);
const settingsSection = ref<SettingsSection>("general");
const activeId = ref(GROUPS_TAB_ID);

function folderName(path: string) {
  const parts = path.split("/").filter(Boolean);
  return parts[parts.length - 1] ?? path;
}

export function isSettingsSection(value: unknown): value is SettingsSection {
  return SETTINGS_SECTIONS.includes(value as SettingsSection);
}

export function settingsPath(section: SettingsSection = settingsSection.value) {
  return section === "general" ? "/settings" : `/settings/${section}`;
}

export function useTabs() {
  const router = useRouter();
  const { findRepo } = useApp();

  const tabs = computed<AppTab[]>(() => [
    { id: GROUPS_TAB_ID, title: "Repositories", closable: false },
    ...repoTabs.value.map((tab) => {
      const match = findRepo(tab.id);
      return {
        ...tab,
        closable: true,
        accentColor: match?.group?.headerColor || match?.repo.headerColor,
      };
    }),
    ...(historyTabOpen.value
      ? [{ id: HISTORY_TAB_ID, title: "History", closable: true }]
      : []),
    ...(settingsTabOpen.value
      ? [{ id: SETTINGS_TAB_ID, title: "Settings", closable: true }]
      : []),
    ...(changelogTabOpen.value
      ? [{ id: CHANGELOG_TAB_ID, title: "Change Log", closable: true }]
      : []),
  ]);

  function titleFor(id: string) {
    const match = findRepo(id);
    if (match?.status?.name) {
      return match.status.name;
    }
    if (match?.repo.path) {
      return folderName(match.repo.path);
    }
    return id;
  }

  function ensureTab(id: string) {
    if (repoTabs.value.some((tab) => tab.id === id)) {
      return;
    }
    repoTabs.value = [...repoTabs.value, { id, title: titleFor(id) }];
  }

  function routeFor(id: string) {
    if (id === GROUPS_TAB_ID) {
      return "/";
    }
    if (id === HISTORY_TAB_ID) {
      return "/history";
    }
    if (id === SETTINGS_TAB_ID) {
      return settingsPath();
    }
    if (id === CHANGELOG_TAB_ID) {
      return "/changelog";
    }
    return `/repo/${id}`;
  }

  function activate(id: string) {
    activeId.value = id;
    const target = routeFor(id);
    if (router.currentRoute.value.fullPath !== target) {
      void router.push(target);
    }
  }

  function openRepo(id: string) {
    ensureTab(id);
    activate(id);
  }

  function openRepos(ids: string[], focusId = ids[ids.length - 1]) {
    for (const id of ids) {
      ensureTab(id);
    }
    if (focusId) {
      activate(focusId);
    }
  }

  function fallbackUtilityId(closingId: string) {
    if (closingId !== HISTORY_TAB_ID && historyTabOpen.value) {
      return HISTORY_TAB_ID;
    }
    if (closingId !== SETTINGS_TAB_ID && settingsTabOpen.value) {
      return SETTINGS_TAB_ID;
    }
    if (closingId !== CHANGELOG_TAB_ID && changelogTabOpen.value) {
      return CHANGELOG_TAB_ID;
    }
    return repoTabs.value[repoTabs.value.length - 1]?.id ?? GROUPS_TAB_ID;
  }

  function closeUtilityTab(id: string, open: { value: boolean }) {
    if (!open.value) {
      return;
    }
    const wasActive = activeId.value === id;
    open.value = false;
    if (wasActive) {
      activate(fallbackUtilityId(id));
    }
  }

  function closeHistory() {
    closeUtilityTab(HISTORY_TAB_ID, historyTabOpen);
  }

  function closeSettings() {
    closeUtilityTab(SETTINGS_TAB_ID, settingsTabOpen);
  }

  function closeChangelog() {
    closeUtilityTab(CHANGELOG_TAB_ID, changelogTabOpen);
  }

  function openHistory() {
    historyTabOpen.value = true;
    activate(HISTORY_TAB_ID);
  }

  function openSettings(section: SettingsSection = "general") {
    settingsTabOpen.value = true;
    settingsSection.value = section;
    activate(SETTINGS_TAB_ID);
  }

  function openSettingsJson() {
    openSettings("json");
  }

  function openChangelog() {
    changelogTabOpen.value = true;
    activate(CHANGELOG_TAB_ID);
  }

  function closeActiveTab() {
    const tab = tabs.value.find((item) => item.id === activeId.value);
    if (!tab?.closable) {
      return false;
    }
    closeRepo(tab.id);
    return true;
  }

  function closeRepo(id: string) {
    if (id === GROUPS_TAB_ID) {
      return;
    }
    if (id === HISTORY_TAB_ID) {
      closeHistory();
      return;
    }
    if (id === SETTINGS_TAB_ID) {
      closeSettings();
      return;
    }
    if (id === CHANGELOG_TAB_ID) {
      closeChangelog();
      return;
    }
    const index = repoTabs.value.findIndex((tab) => tab.id === id);
    if (index === -1) {
      return;
    }
    repoTabs.value = repoTabs.value.filter((tab) => tab.id !== id);
    if (activeId.value === id) {
      const neighbor = repoTabs.value[index] ?? repoTabs.value[index - 1];
      activate(neighbor?.id ?? GROUPS_TAB_ID);
    }
  }

  function closeRepos(ids: string[]) {
    const closing = new Set(ids);
    const index = repoTabs.value.findIndex((tab) => tab.id === activeId.value);
    repoTabs.value = repoTabs.value.filter((tab) => !closing.has(tab.id));
    if (closing.has(activeId.value)) {
      const neighbor = repoTabs.value[index] ?? repoTabs.value[index - 1];
      activate(neighbor?.id ?? GROUPS_TAB_ID);
    }
  }

  function hasTab(id: string) {
    if (id === HISTORY_TAB_ID) {
      return historyTabOpen.value;
    }
    if (id === SETTINGS_TAB_ID) {
      return settingsTabOpen.value;
    }
    if (id === CHANGELOG_TAB_ID) {
      return changelogTabOpen.value;
    }
    return repoTabs.value.some((tab) => tab.id === id);
  }

  function syncFromRoute(
    repoId: string | undefined,
    isHome: boolean,
    panel?: UtilityPanel,
    section?: string,
  ) {
    if (isHome) {
      activeId.value = GROUPS_TAB_ID;
      return;
    }
    if (panel === "history") {
      historyTabOpen.value = true;
      activeId.value = HISTORY_TAB_ID;
      return;
    }
    if (panel === "settings") {
      settingsTabOpen.value = true;
      settingsSection.value = isSettingsSection(section) ? section : "general";
      activeId.value = SETTINGS_TAB_ID;
      return;
    }
    if (panel === "changelog") {
      changelogTabOpen.value = true;
      activeId.value = CHANGELOG_TAB_ID;
      return;
    }
    if (!repoId) {
      return;
    }
    ensureTab(repoId);
    activeId.value = repoId;
  }

  function refreshTitles() {
    repoTabs.value = repoTabs.value.map((tab) => ({
      ...tab,
      title: titleFor(tab.id),
    }));
  }

  return {
    tabs,
    repoTabs,
    historyTabOpen,
    settingsTabOpen,
    changelogTabOpen,
    settingsSection,
    activeId,
    openRepo,
    openRepos,
    openHistory,
    openSettings,
    openSettingsJson,
    openChangelog,
    activate,
    closeRepo,
    closeActiveTab,
    closeRepos,
    hasTab,
    syncFromRoute,
    refreshTitles,
  };
}
