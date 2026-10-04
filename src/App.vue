<script setup lang="ts">
import { computed, defineAsyncComponent, h, onMounted, onUnmounted, ref, watch } from "vue";
import { getVersion } from "@tauri-apps/api/app";
import { listen } from "@tauri-apps/api/event";
import { openUrl } from "@tauri-apps/plugin-opener";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useRoute } from "vue-router";
import TabBar from "./components/TabBar.vue";
import RepoPane from "./components/RepoPane.vue";
import OutputModal from "./components/OutputModal.vue";
import Toast from "./components/Toast.vue";
import Modal from "./components/Modal.vue";
import GroupsView from "./views/GroupsView.vue";

const HistoryView = defineAsyncComponent(() => import("./views/HistoryView.vue"));
const SettingsView = defineAsyncComponent({
  loader: () => import("./views/SettingsView.vue"),
  errorComponent: {
    setup() {
      return () =>
        h("p", { class: "settings-error settings-pane-error" }, "Could not open Settings.");
    },
  },
});
const ChangelogView = defineAsyncComponent(() => import("./views/ChangelogView.vue"));
import { useApp } from "./composables/useApp";
import { resolveFontStack } from "./fonts";
import { watchWindowFocus } from "./notifications";
import { useUpdater } from "./composables/useUpdater";
import {
  CHANGELOG_TAB_ID,
  GROUPS_TAB_ID,
  HISTORY_TAB_ID,
  SETTINGS_TAB_ID,
  useTabs,
} from "./composables/useTabs";

const route = useRoute();
const {
  load,
  error,
  statuses,
  toastMessage,
  toastKind,
  toastHasDetails,
  actionOutput,
  actionOutputOpen,
  dismissToast,
  pauseToast,
  resumeToast,
  dismissOutput,
  openOutput,
  showToast,
  diffFontFamily,
  diffFontSize,
  terminalFontFamily,
  terminalFontSize,
  countdownLabel,
  lastRefreshAt,
} = useApp();

const lastFetchTitle = computed(() => {
  if (!lastRefreshAt.value) {
    return "No fetch yet";
  }
  const time = lastRefreshAt.value.toLocaleTimeString([], { hour: "numeric", minute: "2-digit" });
  return `Last fetch ${time}`;
});

const appStyle = computed(() => ({
  "--diff-font-family": resolveFontStack(diffFontFamily.value),
  "--diff-font-size": `${diffFontSize.value}px`,
  "--terminal-font-family": resolveFontStack(terminalFontFamily.value),
  "--terminal-font-size": `${terminalFontSize.value}px`,
}));
const {
  status,
  statusText,
  currentVersion,
  availableVersion,
  notes,
  busy,
  promptOpen,
  checkForUpdates,
  installUpdate,
  dismissPrompt,
} = useUpdater();

function onToastDetails() {
  openOutput();
  dismissToast();
}
const GITHUB_URL = "https://github.com/fylzero/shipyard";
const appVersion = ref("1.1.0");
const {
  repoTabs,
  historyTabOpen,
  settingsTabOpen,
  changelogTabOpen,
  activeId,
  syncFromRoute,
  refreshTitles,
  closeActiveTab,
  openChangelog,
  openSettings,
} = useTabs();

let stopCloseShortcut: (() => void) | undefined;
let stopOpenSettings: (() => void) | undefined;
let stopCheckForUpdates: (() => void) | undefined;

async function closeActiveTabOrWindow() {
  if (closeActiveTab()) {
    return;
  }
  await getCurrentWindow().close();
}

async function onMenuCheckForUpdates() {
  await checkForUpdates({ prompt: true });
  if (status.value === "up-to-date") {
    showToast("You're on the latest version.");
  } else if (status.value === "error") {
    showToast(statusText.value, "error");
  }
}

async function openGitHub() {
  try {
    await openUrl(GITHUB_URL);
  } catch {
    /* keep the status bar quiet if the OS cannot open the URL */
  }
}

function onWindowKeydown(event: KeyboardEvent) {
  if (!(event.metaKey || event.ctrlKey) || event.altKey || event.shiftKey) {
    return;
  }
  if (event.key.toLowerCase() !== "w") {
    return;
  }
  if (!closeActiveTab()) {
    return;
  }
  event.preventDefault();
  event.stopPropagation();
}

onMounted(() => {
  watchWindowFocus();
  void load();
  void getVersion()
    .then((value) => {
      appVersion.value = value;
    })
    .catch(() => {
      /* keep the bundled fallback */
    });
  window.addEventListener("keydown", onWindowKeydown, true);
  void listen("close-tab-or-window", () => {
    void closeActiveTabOrWindow();
  }).then((unlisten) => {
    stopCloseShortcut = unlisten;
  });
  void listen("open-settings", () => {
    openSettings();
  }).then((unlisten) => {
    stopOpenSettings = unlisten;
  });
  void listen("check-for-updates", () => {
    void onMenuCheckForUpdates();
  }).then((unlisten) => {
    stopCheckForUpdates = unlisten;
  });
  if (import.meta.env.PROD) {
    void checkForUpdates({ prompt: true, silent: true });
  }
});

onUnmounted(() => {
  window.removeEventListener("keydown", onWindowKeydown, true);
  stopCloseShortcut?.();
  stopOpenSettings?.();
  stopCheckForUpdates?.();
});

watch(
  () => [route.name, route.params.id, route.params.section],
  () => {
    const repoId = typeof route.params.id === "string" ? route.params.id : undefined;
    const section = typeof route.params.section === "string" ? route.params.section : undefined;
    const panel =
      route.name === "history"
        ? "history"
        : route.name === "settings"
          ? "settings"
          : route.name === "changelog"
            ? "changelog"
            : undefined;
    syncFromRoute(repoId, route.name === "home", panel, section);
  },
  { immediate: true },
);

watch(statuses, () => {
  refreshTitles();
});
</script>

<template>
  <div class="app-shell" :style="appStyle">
    <TabBar />
    <main class="main">
      <p v-if="error" class="banner">{{ error }}</p>
      <div class="main-pane" v-show="activeId === GROUPS_TAB_ID">
        <GroupsView />
      </div>
      <div
        v-for="tab in repoTabs"
        :key="tab.id"
        class="main-pane"
        v-show="activeId === tab.id"
      >
        <RepoPane :repo-id="tab.id" />
      </div>
      <div v-if="historyTabOpen" class="main-pane" v-show="activeId === HISTORY_TAB_ID">
        <HistoryView />
      </div>
      <div v-if="settingsTabOpen" class="main-pane" v-show="activeId === SETTINGS_TAB_ID">
        <SettingsView />
      </div>
      <div v-if="changelogTabOpen" class="main-pane" v-show="activeId === CHANGELOG_TAB_ID">
        <ChangelogView />
      </div>
    </main>
    <footer class="status-bar">
      <div class="status-bar-start">
        <button
          class="status-bar-github"
          type="button"
          title="Open Shipyard on GitHub"
          @click="openGitHub"
        >
          <svg viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
            <path
              d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27s1.36.09 2 .27c1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.01 8.01 0 0 0 16 8c0-4.42-3.58-8-8-8"
            />
          </svg>
        </button>
        <span v-if="countdownLabel" class="status-bar-fetch" :title="lastFetchTitle">{{
          countdownLabel
        }}</span>
      </div>
      <button
        class="status-bar-version"
        type="button"
        :class="{ active: activeId === CHANGELOG_TAB_ID }"
        title="Open the change log"
        @click="openChangelog"
      >
        <span class="status-bar-version-prefix">v</span>{{ appVersion }}
      </button>
    </footer>
    <Transition name="toast" :duration="{ enter: 520, leave: 280 }">
      <Toast
        v-if="toastMessage"
        :message="toastMessage"
        :kind="toastKind"
        :has-details="toastHasDetails"
        @dismiss="dismissToast"
        @details="onToastDetails"
        @pause="pauseToast"
        @resume="resumeToast"
      />
    </Transition>
    <OutputModal
      v-if="actionOutputOpen && actionOutput"
      :title="actionOutput.title"
      :results="actionOutput.results"
      @close="dismissOutput"
    />
    <Modal
      v-if="promptOpen"
      title="Update available"
      @close="busy ? undefined : dismissPrompt()"
    >
      <p>
        Shipyard {{ availableVersion }} is available. You have {{ currentVersion }}.
      </p>
      <p v-if="notes" class="muted tiny settings-update-notes">{{ notes }}</p>
      <p
        v-if="status === 'downloading' || status === 'installing' || status === 'error'"
        class="muted tiny"
        :class="{ 'settings-error': status === 'error' }"
      >
        {{ statusText }}
      </p>
      <template #actions>
        <button class="ghost" type="button" :disabled="busy" @click="dismissPrompt">
          Later
        </button>
        <button class="ghost commit" type="button" :disabled="busy" @click="installUpdate">
          Install and restart
        </button>
      </template>
    </Modal>
  </div>
</template>
