<script setup lang="ts">
import { getVersion } from "@tauri-apps/api/app";
import { openUrl } from "@tauri-apps/plugin-opener";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { onBeforeUnmount, onMounted, ref } from "vue";
import {
  CHANGELOG_TAB_ID,
  GROUPS_TAB_ID,
  HISTORY_TAB_ID,
  SETTINGS_TAB_ID,
  useTabs,
} from "../composables/useTabs";
import { useUpdater } from "../composables/useUpdater";
import { isMac } from "../platform";
import RepoIcon from "./RepoIcon.vue";

const {
  tabs,
  activeId,
  activate,
  closeRepo,
  openChangelog,
  openHistory,
  openSettings,
  isReorderable,
  moveTab,
} = useTabs();
const { updateReady, availableVersion, showPrompt } = useUpdater();

const GITHUB_URL = "https://github.com/fylzero/shipyard";
const appVersion = ref("1.1.0");

void getVersion()
  .then((value) => {
    appVersion.value = value;
  })
  .catch(() => {
    /* keep the bundled fallback */
  });

async function openGitHub() {
  try {
    await openUrl(GITHUB_URL);
  } catch {
    /* stay quiet if the OS cannot open the URL */
  }
}

const DRAG_THRESHOLD = 5;
const tabList = ref<HTMLElement | null>(null);
const draggingId = ref<string | null>(null);
let pressed: { id: string; x: number } | null = null;

function startReorder(event: PointerEvent, id: string) {
  if (event.button !== 0 || !isReorderable(id) || (event.target as Element).closest("button")) {
    return;
  }
  event.preventDefault();
  pressed = { id, x: event.clientX };
  window.addEventListener("pointermove", onReorderMove);
  window.addEventListener("pointerup", endReorder);
  window.addEventListener("pointercancel", endReorder);
}

function onReorderMove(event: PointerEvent) {
  if (!pressed) {
    return;
  }
  if (!draggingId.value) {
    if (Math.abs(event.clientX - pressed.x) < DRAG_THRESHOLD) {
      return;
    }
    draggingId.value = pressed.id;
    activate(pressed.id);
  }
  const dragging = draggingId.value;
  const elements = Array.from(tabList.value?.querySelectorAll<HTMLElement>("[data-reorder-id]") ?? []);
  const target = elements.find((element) => {
    const rect = element.getBoundingClientRect();
    return event.clientX >= rect.left && event.clientX < rect.right;
  });
  const targetId = target?.dataset.reorderId;
  if (!target || !targetId || targetId === dragging) {
    return;
  }
  /**
   * Only swap once the pointer crosses the target's midpoint, so tabs of
   * different widths don't flip back and forth under the cursor.
   */
  const rect = target.getBoundingClientRect();
  const midpoint = rect.left + rect.width / 2;
  const ids = tabs.value.map((tab) => tab.id);
  const movingRight = ids.indexOf(targetId) > ids.indexOf(dragging);
  if (movingRight ? event.clientX > midpoint : event.clientX < midpoint) {
    moveTab(dragging, targetId);
  }
}

function endReorder() {
  pressed = null;
  draggingId.value = null;
  window.removeEventListener("pointermove", onReorderMove);
  window.removeEventListener("pointerup", endReorder);
  window.removeEventListener("pointercancel", endReorder);
}

/**
 * On macOS the window buttons sit over the left of the tab bar, so the bar
 * leaves room for them, except in full screen, where macOS hides them.
 */
const fullscreen = ref(false);
let stopWatchingResize: (() => void) | null = null;

async function syncFullscreen() {
  fullscreen.value = await getCurrentWindow().isFullscreen().catch(() => false);
}

onMounted(async () => {
  if (!isMac) {
    return;
  }
  await syncFullscreen();
  stopWatchingResize = await getCurrentWindow().onResized(syncFullscreen).catch(() => null);
});

onBeforeUnmount(() => {
  endReorder();
  stopWatchingResize?.();
  stopWatchingResize = null;
});
</script>

<template>
  <div class="tab-bar" role="tablist" aria-label="Open views" data-tauri-drag-region>
    <div v-if="isMac && !fullscreen" class="window-controls-space" data-tauri-drag-region />
    <div
      ref="tabList"
      class="tab-list"
      :class="{ reordering: draggingId }"
      data-tauri-drag-region
    >
      <div
        v-for="tab in tabs"
        :key="tab.id"
        class="tab"
        :class="{
          active: activeId === tab.id,
          pinned: !tab.closable,
          dragging: draggingId === tab.id,
          'has-group-color': Boolean(tab.accentColor),
        }"
        :style="tab.accentColor ? { '--tab-accent': tab.accentColor } : undefined"
        :data-reorder-id="isReorderable(tab.id) ? tab.id : undefined"
        role="tab"
        :aria-selected="activeId === tab.id"
        @click="activate(tab.id)"
        @pointerdown="startReorder($event, tab.id)"
      >
        <svg
          v-if="tab.id === GROUPS_TAB_ID"
          class="tab-icon"
          viewBox="0 0 24 24"
          aria-hidden="true"
        >
          <path
            d="M6 6.878V6a2.25 2.25 0 0 1 2.25-2.25h7.5A2.25 2.25 0 0 1 18 6v.878m-12 0c.235-.083.487-.128.75-.128h10.5c.263 0 .515.045.75.128m-12 0A2.25 2.25 0 0 0 4.5 9v.878m13.5-3A2.25 2.25 0 0 1 19.5 9v.878m0 0a2.246 2.246 0 0 0-.75-.128H5.25c-.263 0-.515.045-.75.128m15 0A2.25 2.25 0 0 1 21 12v6a2.25 2.25 0 0 1-2.25 2.25H5.25A2.25 2.25 0 0 1 3 18v-6c0-.98.626-1.813 1.5-2.122"
          />
        </svg>
        <svg
          v-else-if="tab.id === HISTORY_TAB_ID"
          class="tab-icon"
          viewBox="0 0 24 24"
          aria-hidden="true"
        >
          <path d="M12 8v4l3 1.5" />
          <path d="M3.05 11a9 9 0 1 0 .5-3.5" />
          <path d="M3 4.5V8h3.5" />
        </svg>
        <svg
          v-else-if="tab.id === SETTINGS_TAB_ID"
          class="tab-icon"
          viewBox="0 0 24 24"
          aria-hidden="true"
        >
          <path
            d="M9.594 3.94c.09-.542.56-.94 1.11-.94h2.593c.55 0 1.02.398 1.11.94l.213 1.281c.063.374.313.686.645.87.074.04.147.083.22.127.325.196.72.257 1.075.124l1.217-.456a1.125 1.125 0 0 1 1.37.49l1.296 2.247a1.125 1.125 0 0 1-.26 1.431l-1.003.827c-.293.241-.438.613-.43.992a7.723 7.723 0 0 1 0 .255c-.008.378.137.75.43.991l1.004.827c.424.35.534.955.26 1.43l-1.298 2.247a1.125 1.125 0 0 1-1.369.491l-1.217-.456c-.355-.133-.75-.072-1.076.124a6.47 6.47 0 0 1-.22.128c-.331.183-.581.495-.644.869l-.213 1.281c-.09.543-.56.94-1.11.94h-2.594c-.55 0-1.019-.398-1.11-.94l-.213-1.281c-.062-.374-.312-.686-.644-.87a6.52 6.52 0 0 1-.22-.127c-.325-.196-.72-.257-1.076-.124l-1.217.456a1.125 1.125 0 0 1-1.369-.49l-1.297-2.247a1.125 1.125 0 0 1 .26-1.431l1.004-.827c.292-.24.437-.613.43-.991a6.932 6.932 0 0 1 0-.255c.007-.38-.138-.751-.43-.992l-1.004-.827a1.125 1.125 0 0 1-.26-1.43l1.297-2.247a1.125 1.125 0 0 1 1.37-.491l1.216.456c.356.133.751.072 1.076-.124.072-.044.146-.087.22-.128.332-.183.582-.495.644-.869l.214-1.28Z"
          />
          <path d="M15 12a3 3 0 1 1-6 0 3 3 0 0 1 6 0Z" />
        </svg>
        <svg
          v-else-if="tab.id === CHANGELOG_TAB_ID"
          class="tab-icon"
          viewBox="0 0 24 24"
          aria-hidden="true"
        >
          <path
            d="M19.5 14.25v-2.625a3.375 3.375 0 0 0-3.375-3.375h-1.5A1.125 1.125 0 0 1 13.5 7.125v-1.5a3.375 3.375 0 0 0-3.375-3.375H8.25m0 12.75h7.5m-7.5 3H12M10.5 2.25H5.625c-.621 0-1.125.504-1.125 1.125v17.25c0 .621.504 1.125 1.125 1.125h12.75c.621 0 1.125-.504 1.125-1.125V11.25a9 9 0 0 0-9-9Z"
          />
        </svg>
        <RepoIcon v-else class="tab-icon" />
        <span class="tab-title">{{ tab.title }}</span>
        <span v-if="tab.label" class="repo-label tab-label" :title="tab.label">{{ tab.label }}</span>
        <button
          v-if="tab.closable"
          class="tab-close"
          type="button"
          :aria-label="`Close ${tab.title}`"
          @click.stop="closeRepo(tab.id)"
        >
          ×
        </button>
      </div>
    </div>
    <div class="tab-tools">
      <button
        v-if="updateReady"
        class="tab-tool update"
        type="button"
        :title="availableVersion ? `Update to v${availableVersion}` : 'Update available'"
        :aria-label="availableVersion ? `Update to v${availableVersion}` : 'Update available'"
        @click="showPrompt"
      >
        <svg viewBox="0 0 24 24" aria-hidden="true">
          <path d="M12 4v10" />
          <path d="m8 10 4 4 4-4" />
          <path d="M5 18h14" />
        </svg>
      </button>
      <button
        class="tab-tool"
        :class="{ active: activeId === HISTORY_TAB_ID }"
        type="button"
        title="History"
        aria-label="History"
        @click="openHistory"
      >
        <svg viewBox="0 0 24 24" aria-hidden="true">
          <path d="M12 8v4l3 1.5" />
          <path d="M3.05 11a9 9 0 1 0 .5-3.5" />
          <path d="M3 4.5V8h3.5" />
        </svg>
      </button>
      <button
        class="tab-version"
        :class="{ active: activeId === CHANGELOG_TAB_ID }"
        type="button"
        title="Open the change log"
        @click="openChangelog"
      >
        <span class="tab-version-prefix">v</span>{{ appVersion }}
      </button>
      <button
        class="tab-tool"
        type="button"
        title="Open Shipyard on GitHub"
        aria-label="Open Shipyard on GitHub"
        @click="openGitHub"
      >
        <svg class="tab-tool-github" viewBox="0 0 16 16" aria-hidden="true">
          <path
            d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27s1.36.09 2 .27c1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.01 8.01 0 0 0 16 8c0-4.42-3.58-8-8-8"
          />
        </svg>
      </button>
      <button
        class="tab-tool"
        :class="{ active: activeId === SETTINGS_TAB_ID }"
        type="button"
        title="Settings"
        aria-label="Settings"
        @click="openSettings()"
      >
      <svg viewBox="0 0 24 24" aria-hidden="true">
        <path
          d="M9.594 3.94c.09-.542.56-.94 1.11-.94h2.593c.55 0 1.02.398 1.11.94l.213 1.281c.063.374.313.686.645.87.074.04.147.083.22.127.325.196.72.257 1.075.124l1.217-.456a1.125 1.125 0 0 1 1.37.49l1.296 2.247a1.125 1.125 0 0 1-.26 1.431l-1.003.827c-.293.241-.438.613-.43.992a7.723 7.723 0 0 1 0 .255c-.008.378.137.75.43.991l1.004.827c.424.35.534.955.26 1.43l-1.298 2.247a1.125 1.125 0 0 1-1.369.491l-1.217-.456c-.355-.133-.75-.072-1.076.124a6.47 6.47 0 0 1-.22.128c-.331.183-.581.495-.644.869l-.213 1.281c-.09.543-.56.94-1.11.94h-2.594c-.55 0-1.019-.398-1.11-.94l-.213-1.281c-.062-.374-.312-.686-.644-.87a6.52 6.52 0 0 1-.22-.127c-.325-.196-.72-.257-1.076-.124l-1.217.456a1.125 1.125 0 0 1-1.369-.49l-1.297-2.247a1.125 1.125 0 0 1 .26-1.431l1.004-.827c.292-.24.437-.613.43-.991a6.932 6.932 0 0 1 0-.255c.007-.38-.138-.751-.43-.992l-1.004-.827a1.125 1.125 0 0 1-.26-1.43l1.297-2.247a1.125 1.125 0 0 1 1.37-.491l1.216.456c.356.133.751.072 1.076-.124.072-.044.146-.087.22-.128.332-.183.582-.495.644-.869l.214-1.28Z"
        />
        <path d="M15 12a3 3 0 1 1-6 0 3 3 0 0 1 6 0Z" />
      </svg>
    </button>
    </div>
  </div>
</template>
