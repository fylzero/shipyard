<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref } from "vue";
import { listen } from "@tauri-apps/api/event";
import { confirm } from "@tauri-apps/plugin-dialog";
import * as api from "../api";
import { baseName } from "../platform";
import type { CommandLogEntry } from "../types";

const entries = ref<CommandLogEntry[]>([]);
const message = ref("");
const clearing = ref(false);
const showDetail = ref(false);
const hideStatus = ref(true);
const paused = ref(false);
const terminal = ref<HTMLDivElement | null>(null);
let stopLog: (() => void) | undefined;
let stopCleared: (() => void) | undefined;
let stickToBottom = true;

function isStatusNoise(entry: CommandLogEntry) {
  const command = entry.args[0] ?? "";
  if (command === "status" || command === "rev-parse" || command === "show-ref") {
    return true;
  }
  if (command === "ls-files" || command === "rev-list") {
    return true;
  }
  if (command === "diff" && (entry.args.includes("--numstat") || entry.args.includes("--quiet"))) {
    return true;
  }
  return false;
}

const visibleEntries = computed(() =>
  hideStatus.value ? entries.value.filter((entry) => !isStatusNoise(entry)) : entries.value,
);

function formatStamp(at: number) {
  return new Date(at).toLocaleString(undefined, { hour12: false });
}

function onScroll() {
  const node = terminal.value;
  if (!node) {
    return;
  }
  stickToBottom = node.scrollHeight - node.scrollTop - node.clientHeight < 48;
}

async function scrollIfNeeded() {
  if (!stickToBottom) {
    return;
  }
  await nextTick();
  const node = terminal.value;
  if (node) {
    node.scrollTop = node.scrollHeight;
  }
}

onMounted(async () => {
  try {
    const [history, isPaused] = await Promise.all([
      api.commandHistory(),
      api.commandHistoryPaused(),
    ]);
    entries.value = history;
    paused.value = isPaused;
    await scrollIfNeeded();
  } catch (err) {
    message.value = String(err);
  }
  void listen<CommandLogEntry>("command-log", (event) => {
    if (paused.value || entries.value.some((entry) => entry.id === event.payload.id)) {
      return;
    }
    entries.value = [...entries.value, event.payload];
    void scrollIfNeeded();
  }).then((unlisten) => {
    stopLog = unlisten;
  });
  void listen("command-log-cleared", () => {
    entries.value = [];
  }).then((unlisten) => {
    stopCleared = unlisten;
  });
});

onUnmounted(() => {
  stopLog?.();
  stopCleared?.();
});

async function togglePaused() {
  const next = !paused.value;
  message.value = "";
  try {
    await api.setCommandHistoryPaused(next);
    paused.value = next;
  } catch (err) {
    message.value = String(err);
  }
}

async function clearLogs() {
  if (!entries.value.length || clearing.value) {
    return;
  }
  const ok = await confirm("Clear the local git command history? This cannot be undone.", {
    title: "Clear history",
    kind: "warning",
    okLabel: "Clear",
    cancelLabel: "Cancel",
  });
  if (!ok) {
    return;
  }
  clearing.value = true;
  message.value = "";
  try {
    await api.clearCommandHistory();
    entries.value = [];
  } catch (err) {
    message.value = String(err);
  } finally {
    clearing.value = false;
  }
}
</script>

<template>
  <div class="settings-page history-page">
    <div class="settings-inner">
      <div class="settings-header">
        <div>
          <div class="brand">History</div>
          <p class="muted tiny">
            {{
              paused
                ? "Recording paused. Git commands are not written here until you resume."
                : "Every git command Shipyard runs is recorded here, including fetch, pull, push, and commit."
            }}
          </p>
        </div>
        <div class="history-header-actions">
          <div class="segmented" role="group" aria-label="History detail">
            <button
              type="button"
              :class="{ active: !showDetail }"
              :aria-pressed="!showDetail"
              @click="showDetail = false"
            >
              Commands
            </button>
            <button
              type="button"
              :class="{ active: showDetail }"
              :aria-pressed="showDetail"
              @click="showDetail = true"
            >
              Full detail
            </button>
          </div>
          <button
            class="history-switch"
            :class="{ on: hideStatus }"
            type="button"
            role="switch"
            :aria-checked="hideStatus"
            title="Hide refresh status checks like git status, rev-parse, and numstat"
            @click="hideStatus = !hideStatus"
          >
            <span class="history-switch-track" aria-hidden="true">
              <span class="history-switch-knob" />
            </span>
            Hide status
          </button>
          <button
            class="ghost"
            type="button"
            :aria-pressed="paused"
            :title="
              paused
                ? 'Start recording git commands again'
                : 'Stop recording git commands until you resume'
            "
            @click="togglePaused"
          >
            <svg v-if="paused" class="button-icon" viewBox="0 0 24 24" aria-hidden="true">
              <path d="M9 7.2v9.6l8.2-4.8Z" stroke-linejoin="round" />
            </svg>
            <svg v-else class="button-icon" viewBox="0 0 24 24" aria-hidden="true">
              <rect x="7" y="6" width="3" height="12" rx="0.75" />
              <rect x="14" y="6" width="3" height="12" rx="0.75" />
            </svg>
            {{ paused ? "Resume" : "Pause" }}
          </button>
          <button
            class="ghost danger"
            type="button"
            :disabled="clearing || !entries.length"
            @click="clearLogs"
          >
            {{ clearing ? "Clearing…" : "Clear logs" }}
          </button>
        </div>
      </div>
      <div ref="terminal" class="history-terminal" @scroll="onScroll">
        <p v-if="paused" class="history-paused-banner">Recording paused</p>
        <p v-if="!entries.length" class="muted tiny history-empty">
          {{
            paused
              ? "Resume to capture git commands again."
              : "No commands yet. Fetch a group or open a repo to see each git invocation."
          }}
        </p>
        <p v-else-if="!visibleEntries.length" class="muted tiny history-empty">
          Only status checks are in this log. Turn off Hide status to see them.
        </p>
        <article
          v-for="entry in visibleEntries"
          :key="entry.id"
          class="history-entry"
          :class="{ bad: !entry.success, compact: !showDetail }"
        >
          <header class="history-meta">
            <span class="history-time">{{ formatStamp(entry.at) }}</span>
            <span class="history-cwd" :title="entry.cwd">{{ baseName(entry.cwd) }}</span>
            <span class="history-status">
              {{ entry.success ? "exit 0" : "failed" }} · {{ entry.durationMs }}ms
            </span>
          </header>
          <pre class="history-command" :title="entry.program">$ {{ entry.command }}</pre>
          <template v-if="showDetail">
            <pre v-if="entry.stdout.trim()" class="history-output">{{ entry.stdout }}</pre>
            <pre v-if="entry.stderr.trim()" class="history-output err">{{ entry.stderr }}</pre>
          </template>
        </article>
      </div>
      <p v-if="message" class="settings-error">{{ message }}</p>
    </div>
  </div>
</template>
