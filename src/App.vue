<script setup lang="ts">
import { computed, defineAsyncComponent, h, onMounted, onUnmounted, watch } from "vue";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useRoute } from "vue-router";
import TabBar from "./components/TabBar.vue";
import RepoPane from "./components/RepoPane.vue";
import OutputModal from "./components/OutputModal.vue";
import Toast from "./components/Toast.vue";
import Modal from "./components/Modal.vue";
import BranchConfirmModal from "./components/BranchConfirmModal.vue";
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
import { useBranchConfirm } from "./composables/useBranchConfirm";
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
} = useApp();
const { request: branchConfirm, answer: answerBranchConfirm } = useBranchConfirm();

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
const {
  repoPanes,
  historyTabOpen,
  settingsTabOpen,
  changelogTabOpen,
  activeId,
  syncFromRoute,
  refreshTitles,
  closeActiveTab,
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
        v-for="tab in repoPanes"
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
    <BranchConfirmModal
      v-if="branchConfirm"
      v-bind="branchConfirm"
      @answer="answerBranchConfirm"
    />
    <Modal
      v-if="promptOpen"
      title="Update available"
      @close="busy ? undefined : dismissPrompt()"
    >
      <p>
        Shipyard {{ availableVersion }} is available. You have {{ currentVersion }}.
      </p>
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
