<script setup lang="ts">
import { HighlightStyle, syntaxHighlighting } from "@codemirror/language";
import { EditorView } from "@codemirror/view";
import { tags as t } from "@lezer/highlight";
import { basicSetup } from "codemirror";
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import * as api from "../../api";
import { useApp } from "../../composables/useApp";
import { SETTINGS_TAB_ID, useTabs } from "../../composables/useTabs";
import type { GitConfig } from "../../types";

const { showToast } = useApp();
const { activeId, settingsSection } = useTabs();

const editorHost = ref<HTMLDivElement | null>(null);
const userName = ref("");
const userEmail = ref("");
const defaultBranch = ref("");
const pullRebase = ref("");
const defaultRemote = ref("");
const savedName = ref("");
const savedEmail = ref("");
const savedBranch = ref("");
const savedPull = ref("");
const savedRemote = ref("");
const configPath = ref("");
const draft = ref("");
const baseline = ref("");
const message = ref("");
const loading = ref(true);
const savingForm = ref(false);
const savingFile = ref(false);
const revealing = ref(false);
let view: EditorView | null = null;

const formDirty = computed(
  () =>
    userName.value !== savedName.value ||
    userEmail.value !== savedEmail.value ||
    defaultBranch.value !== savedBranch.value ||
    pullRebase.value !== savedPull.value ||
    defaultRemote.value !== savedRemote.value,
);
const fileDirty = computed(() => draft.value !== baseline.value);
const formLocked = computed(() => fileDirty.value || savingFile.value || savingForm.value);
const fileLocked = computed(() => formDirty.value || savingForm.value || savingFile.value);

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
  { tag: t.squareBracket, color: "#7ee0c7" },
  { tag: t.atom, color: "#7ee0c7" },
  { tag: t.propertyName, color: "#9ec1ff" },
  { tag: t.string, color: "#3dd68c" },
  { tag: t.comment, color: "var(--muted)" },
  { tag: t.punctuation, color: "var(--muted)" },
  { tag: t.invalid, color: "var(--bad)" },
]);

const displayPath = computed(() => {
  const path = configPath.value;
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

function assignForm(next: GitConfig) {
  userName.value = next.userName;
  userEmail.value = next.userEmail;
  defaultBranch.value = next.defaultBranch;
  pullRebase.value = next.pullRebase;
  defaultRemote.value = next.defaultRemote ?? "";
  savedName.value = next.userName;
  savedEmail.value = next.userEmail;
  savedBranch.value = next.defaultBranch;
  savedPull.value = next.pullRebase;
  savedRemote.value = next.defaultRemote ?? "";
}

function applyConfig(next: GitConfig, force = false) {
  configPath.value = next.path;
  if (force || !formDirty.value) {
    assignForm(next);
  }
  if (force || !fileDirty.value) {
    setDraft(next.contents);
    baseline.value = next.contents;
  }
}

async function loadConfig(forceFile = false) {
  const next = await api.gitConfig();
  applyConfig(next, forceFile);
}

async function syncIfClean() {
  if (formDirty.value || fileDirty.value || savingForm.value || savingFile.value) {
    return;
  }
  try {
    await loadConfig();
    message.value = "";
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
    await loadConfig(true);
    message.value = "";
  } catch (err) {
    message.value = String(err);
  } finally {
    loading.value = false;
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
  if (id === SETTINGS_TAB_ID && section === "git") {
    void syncIfClean();
    void mountEditor();
    view?.requestMeasure();
  }
});

function discardForm() {
  message.value = "";
  userName.value = savedName.value;
  userEmail.value = savedEmail.value;
  defaultBranch.value = savedBranch.value;
  pullRebase.value = savedPull.value;
  defaultRemote.value = savedRemote.value;
}

async function saveForm() {
  if (formLocked.value || !formDirty.value) {
    return;
  }
  message.value = "";
  savingForm.value = true;
  try {
    const updates: [string, string][] = [];
    if (userName.value !== savedName.value) {
      updates.push(["user.name", userName.value]);
    }
    if (userEmail.value !== savedEmail.value) {
      updates.push(["user.email", userEmail.value]);
    }
    if (defaultBranch.value !== savedBranch.value) {
      updates.push(["init.defaultBranch", defaultBranch.value]);
    }
    if (pullRebase.value !== savedPull.value) {
      updates.push(["pull.rebase", pullRebase.value]);
    }
    if (defaultRemote.value !== savedRemote.value) {
      updates.push(["checkout.defaultRemote", defaultRemote.value]);
    }
    let next: GitConfig | undefined;
    for (const [key, value] of updates) {
      next = await api.updateGitConfigValue(key, value);
    }
    if (next) {
      applyConfig(next, true);
    }
    showToast("Git config has been saved.");
  } catch (err) {
    message.value = String(err);
    showToast(String(err), "error");
  } finally {
    savingForm.value = false;
  }
}

async function discardFile() {
  message.value = "";
  try {
    await loadConfig(true);
  } catch (err) {
    message.value = String(err);
  }
}

async function saveFile() {
  message.value = "";
  savingFile.value = true;
  try {
    applyConfig(await api.saveGitConfigFile(draft.value), true);
    showToast("Git config has been saved.");
  } catch (err) {
    message.value = String(err);
    showToast(String(err), "error");
  } finally {
    savingFile.value = false;
  }
}

async function revealFile() {
  message.value = "";
  revealing.value = true;
  try {
    await api.revealGitConfigFile();
  } catch (err) {
    message.value = String(err);
  } finally {
    revealing.value = false;
  }
}
</script>

<template>
  <div class="settings-pane settings-git-pane">
    <div class="settings-inner">
      <div class="settings-header">
        <div>
          <div class="brand">Git</div>
          <p class="muted tiny">
            Edits go to your global git config. Shipyard uses the same identity as Terminal.
          </p>
        </div>
      </div>

      <section class="settings-card settings-git-fields">
        <div class="settings-git-card-header">
          <div class="settings-header-actions">
            <button
              v-if="formDirty"
              class="ghost"
              type="button"
              :disabled="formLocked"
              @click="discardForm"
            >
              Discard
            </button>
            <button
              class="primary"
              type="button"
              :disabled="formLocked || loading || !formDirty"
              @click="saveForm"
            >
              {{ savingForm ? "Saving…" : "Save" }}
            </button>
          </div>
        </div>
        <div class="settings-row">
          <div class="settings-row-copy">
            <h3>Name</h3>
            <p class="muted tiny">user.name on commits you create in Shipyard or Terminal.</p>
          </div>
          <label class="settings-control settings-text">
            <span class="visually-hidden">Git user name</span>
            <input
              v-model="userName"
              type="text"
              autocapitalize="off"
              autocorrect="off"
              autocomplete="name"
              spellcheck="false"
              :disabled="formLocked || loading"
            />
          </label>
        </div>
        <div class="settings-row">
          <div class="settings-row-copy">
            <h3>Email</h3>
            <p class="muted tiny">user.email. Use the address your remotes expect.</p>
          </div>
          <label class="settings-control settings-text">
            <span class="visually-hidden">Git user email</span>
            <input
              v-model="userEmail"
              type="text"
              autocapitalize="off"
              autocorrect="off"
              autocomplete="email"
              spellcheck="false"
              :disabled="formLocked || loading"
            />
          </label>
        </div>
        <div class="settings-row">
          <div class="settings-row-copy">
            <h3>Default branch</h3>
            <p class="muted tiny">init.defaultBranch for repositories created with git init.</p>
          </div>
          <label class="settings-control settings-text">
            <span class="visually-hidden">Default branch</span>
            <input
              v-model="defaultBranch"
              type="text"
              autocapitalize="off"
              autocorrect="off"
              autocomplete="off"
              spellcheck="false"
              :disabled="formLocked || loading"
              placeholder="main"
            />
          </label>
        </div>
        <div class="settings-row">
          <div class="settings-row-copy">
            <h3>Pull</h3>
            <p class="muted tiny">
              pull.rebase. Merge is rebase = false. Rebase is rebase = true.
            </p>
          </div>
          <label class="settings-control">
            <span class="visually-hidden">Pull behavior</span>
            <select v-model="pullRebase" :disabled="formLocked || loading">
              <option value="">Git default</option>
              <option value="false">Merge</option>
              <option value="true">Rebase</option>
            </select>
          </label>
        </div>
        <div class="settings-row">
          <div class="settings-row-copy">
            <h3>Default remote</h3>
            <p class="muted tiny">
              checkout.defaultRemote when a branch exists on more than one remote.
            </p>
          </div>
          <label class="settings-control settings-text">
            <span class="visually-hidden">Default remote</span>
            <input
              v-model="defaultRemote"
              type="text"
              autocapitalize="off"
              autocorrect="off"
              autocomplete="off"
              spellcheck="false"
              :disabled="formLocked || loading"
              placeholder="origin"
            />
          </label>
        </div>
        <p v-if="fileDirty" class="muted tiny">
          Save or discard the file below to edit these options.
        </p>
      </section>

      <section class="settings-card settings-git-file">
        <div class="settings-git-file-header">
          <div>
            <h3>Config file</h3>
            <p class="muted tiny settings-path" :title="configPath">
              {{
                displayPath ||
                "Your global ~/.gitconfig. Save the form above to create it if it is missing."
              }}
            </p>
          </div>
          <div class="settings-header-actions">
            <button class="ghost" type="button" :disabled="revealing || loading" @click="revealFile">
              {{ revealing ? "Revealing…" : "Reveal" }}
            </button>
            <button
              v-if="fileDirty"
              class="ghost"
              type="button"
              :disabled="fileLocked"
              @click="discardFile"
            >
              Discard
            </button>
            <button
              class="primary"
              type="button"
              :disabled="fileLocked || loading || !fileDirty"
              @click="saveFile"
            >
              {{ savingFile ? "Saving…" : "Save" }}
            </button>
          </div>
        </div>
        <div class="settings-editor">
          <div ref="editorHost" class="settings-json" />
        </div>
        <p v-if="formDirty" class="muted tiny">
          Save or discard the options above to edit the file.
        </p>
        <p v-else-if="fileDirty" class="muted tiny">
          Save or discard the file to edit the options above.
        </p>
      </section>
      <p v-if="message" class="settings-error">{{ message }}</p>
    </div>
  </div>
</template>
