<script setup lang="ts">
import { json } from "@codemirror/lang-json";
import { HighlightStyle, syntaxHighlighting } from "@codemirror/language";
import { EditorView } from "@codemirror/view";
import { tags as t } from "@lezer/highlight";
import { basicSetup } from "codemirror";
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import { confirm, open as openFile, save as saveFile } from "@tauri-apps/plugin-dialog";
import * as api from "../../api";
import { useApp } from "../../composables/useApp";
import { SETTINGS_TAB_ID, useTabs } from "../../composables/useTabs";
import type { AppData } from "../../types";

const {
  groups,
  standaloneRepos,
  dashboardOrder,
  refreshIntervalSeconds,
  refreshActiveHours,
  diffMode,
  diffFontFamily,
  diffFontSize,
  terminalFontFamily,
  terminalFontSize,
  editor,
  notifications,
  windowState,
  replaceSettings,
  showToast,
} = useApp();
const { activeId, settingsSection } = useTabs();

const editorHost = ref<HTMLDivElement | null>(null);
const draft = ref("");
const message = ref("");
const saving = ref(false);
const baseline = ref("");
const copied = ref(false);
const exporting = ref(false);
const exported = ref(false);
const importing = ref(false);
const revealing = ref(false);
const settingsPath = ref("");
let view: EditorView | null = null;

const jsonDirty = computed(() => draft.value !== baseline.value);

const editorTheme = EditorView.theme(
  {
    "&": {
      height: "100%",
      color: "var(--text)",
      backgroundColor: "#0d1016",
      fontFamily: "var(--font-code)",
      fontSize: "12px",
    },
    "&.cm-focused": {
      outline: "none",
    },
    ".cm-scroller": {
      fontFamily: "var(--font-code)",
      lineHeight: "1.5",
    },
    ".cm-content": {
      caretColor: "var(--text)",
      padding: "0.45rem 0.8rem",
    },
    ".cm-gutters": {
      backgroundColor: "#0d1016",
      color: "var(--muted)",
      borderRight: "1px solid var(--border)",
    },
    ".cm-activeLine": {
      backgroundColor: "rgba(255, 255, 255, 0.03)",
    },
    ".cm-activeLineGutter": {
      backgroundColor: "transparent",
    },
    ".cm-selectionBackground, &.cm-focused .cm-selectionBackground": {
      backgroundColor: "#2a3344",
    },
    ".cm-cursor": {
      borderLeftColor: "var(--text)",
    },
  },
  { dark: true },
);

const editorHighlight = HighlightStyle.define([
  { tag: t.propertyName, color: "#7ee0c7" },
  { tag: t.string, color: "#3dd68c" },
  { tag: t.number, color: "#e6c07b" },
  { tag: t.bool, color: "#9ec1ff" },
  { tag: t.null, color: "var(--muted)" },
  { tag: t.punctuation, color: "var(--muted)" },
  { tag: t.squareBracket, color: "#d7deea" },
  { tag: t.brace, color: "#d7deea" },
  { tag: t.invalid, color: "var(--bad)" },
]);

const displayPath = computed(() => {
  const path = settingsPath.value;
  if (!path) {
    return "";
  }
  const home = path.match(/^\/Users\/[^/]+/);
  return home ? path.replace(home[0], "~") : path;
});

function setDraft(value: string) {
  draft.value = value;
  if (!view) {
    return;
  }
  const current = view.state.doc.toString();
  if (current === value) {
    return;
  }
  view.dispatch({
    changes: { from: 0, to: view.state.doc.length, insert: value },
  });
}

function prettyState(data: AppData) {
  return JSON.stringify(data, null, 2);
}

async function loadSavedJson() {
  const pretty = prettyState(await api.getState());
  setDraft(pretty);
  baseline.value = pretty;
}

async function syncJsonIfClean() {
  if (jsonDirty.value) {
    return;
  }
  try {
    await loadSavedJson();
  } catch (err) {
    message.value = String(err);
  }
}

async function mountEditor() {
  await nextTick();
  if (view || !editorHost.value) {
    return;
  }
  view = new EditorView({
    doc: draft.value,
    parent: editorHost.value,
    extensions: [
      basicSetup,
      json(),
      editorTheme,
      syntaxHighlighting(editorHighlight),
      EditorView.updateListener.of((update) => {
        if (update.docChanged) {
          draft.value = update.state.doc.toString();
        }
      }),
    ],
  });
  view.requestMeasure();
}

onMounted(async () => {
  try {
    settingsPath.value = await api.settingsFilePath();
    await loadSavedJson();
  } catch (err) {
    message.value = String(err);
  }
  try {
    await mountEditor();
  } catch (err) {
    message.value = String(err);
  }
});

onUnmounted(() => {
  view?.destroy();
  view = null;
});

watch([activeId, settingsSection], ([id, section]) => {
  if (id === SETTINGS_TAB_ID && section === "json") {
    void syncJsonIfClean();
    void mountEditor();
    view?.requestMeasure();
  }
});

watch(
  [
    groups,
    standaloneRepos,
    dashboardOrder,
    refreshIntervalSeconds,
    refreshActiveHours,
    diffMode,
    diffFontFamily,
    diffFontSize,
    terminalFontFamily,
    terminalFontSize,
    editor,
    notifications,
    windowState,
  ],
  () => {
    void syncJsonIfClean();
  },
);

async function copy() {
  try {
    await navigator.clipboard.writeText(draft.value);
    copied.value = true;
    window.setTimeout(() => {
      copied.value = false;
    }, 1600);
  } catch (err) {
    message.value = String(err);
  }
}

async function exportSettings() {
  message.value = "";
  exported.value = false;
  let contents = draft.value;
  try {
    contents = prettyState(JSON.parse(draft.value) as AppData);
  } catch {
    message.value = "Settings JSON is not valid.";
    return;
  }
  const path = await saveFile({
    title: "Export settings",
    defaultPath: "shipyard-settings.json",
    filters: [{ name: "JSON", extensions: ["json"] }],
  });
  if (!path) {
    return;
  }
  exporting.value = true;
  try {
    await api.writeTextFile(path, `${contents}\n`);
    exported.value = true;
    window.setTimeout(() => {
      exported.value = false;
    }, 1600);
  } catch (err) {
    message.value = String(err);
  } finally {
    exporting.value = false;
  }
}

async function importSettings() {
  message.value = "";
  const path = await openFile({
    title: "Import settings",
    multiple: false,
    filters: [{ name: "JSON", extensions: ["json"] }],
  });
  if (!path || Array.isArray(path)) {
    return;
  }
  importing.value = true;
  try {
    const raw = await api.readTextFile(path);
    const parsed = JSON.parse(raw) as AppData;
    const confirmed = await confirm(
      "Replace all settings, including repositories and groups, with this file?",
      { title: "Import settings", kind: "warning", okLabel: "Import", cancelLabel: "Cancel" },
    );
    if (!confirmed) {
      return;
    }
    const next = await replaceSettings(parsed);
    const pretty = prettyState(next);
    setDraft(pretty);
    baseline.value = pretty;
    showToast("Settings have been imported.");
  } catch (err) {
    if (err instanceof SyntaxError) {
      message.value = "Settings JSON is not valid.";
    } else {
      message.value = String(err);
    }
  } finally {
    importing.value = false;
  }
}

async function revealFile() {
  message.value = "";
  revealing.value = true;
  try {
    await api.revealSettingsFile();
  } catch (err) {
    message.value = String(err);
  } finally {
    revealing.value = false;
  }
}

async function discardJson() {
  message.value = "";
  try {
    await loadSavedJson();
  } catch (err) {
    message.value = String(err);
  }
}

async function save() {
  message.value = "";
  let parsed: AppData;
  try {
    parsed = JSON.parse(draft.value) as AppData;
  } catch {
    message.value = "Settings JSON is not valid.";
    return;
  }
  saving.value = true;
  try {
    const next = await replaceSettings(parsed);
    const pretty = prettyState(next);
    setDraft(pretty);
    baseline.value = pretty;
    showToast("Settings have been saved.");
  } catch (err) {
    message.value = String(err);
  } finally {
    saving.value = false;
  }
}
</script>

<template>
  <div class="settings-pane settings-json-pane">
    <div class="settings-inner">
      <div class="settings-header">
        <div>
          <div class="brand">JSON</div>
          <p class="muted tiny settings-path" :title="settingsPath">
            {{ displayPath || "Edit the saved settings file here, or import a backup." }}
          </p>
        </div>
        <div class="settings-header-actions">
          <button class="ghost" type="button" :disabled="revealing" @click="revealFile">
            {{ revealing ? "Revealing…" : "Reveal" }}
          </button>
          <button class="ghost" type="button" :disabled="importing" @click="importSettings">
            {{ importing ? "Importing…" : "Import" }}
          </button>
          <button
            class="ghost"
            type="button"
            :disabled="exporting || !draft.trim()"
            @click="exportSettings"
          >
            <svg class="button-icon" viewBox="0 0 24 24" aria-hidden="true">
              <path
                d="M9 8.25H7.5a2.25 2.25 0 0 0-2.25 2.25v9a2.25 2.25 0 0 0 2.25 2.25h9a2.25 2.25 0 0 0 2.25-2.25v-9A2.25 2.25 0 0 0 16.5 8.25H15M9 12l3 3m0 0 3-3m-3 3V2.25"
              />
            </svg>
            {{ exporting ? "Exporting…" : exported ? "Exported" : "Export" }}
          </button>
          <button
            v-if="jsonDirty"
            class="ghost"
            type="button"
            :disabled="saving"
            @click="discardJson"
          >
            Discard
          </button>
          <button
            class="primary"
            type="button"
            :disabled="saving || !jsonDirty || !draft.trim()"
            @click="save"
          >
            {{ saving ? "Saving…" : "Save" }}
          </button>
        </div>
      </div>
      <div class="settings-editor">
        <button class="ghost tiny settings-copy" type="button" @click="copy">
          {{ copied ? "Copied" : "Copy" }}
        </button>
        <div ref="editorHost" class="settings-json" />
      </div>
      <p v-if="message" class="settings-error">{{ message }}</p>
    </div>
  </div>
</template>
