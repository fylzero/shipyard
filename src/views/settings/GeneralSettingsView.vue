<script setup lang="ts">
import { ref, watch } from "vue";
import { useApp } from "../../composables/useApp";
import {
  CUSTOM_FONT_ID,
  DEFAULT_DIFF_FONT_SIZE,
  DEFAULT_TERMINAL_FONT_SIZE,
  FONT_OPTIONS,
  FONT_SIZE_MAX,
  FONT_SIZE_MIN,
  formatFontSize,
  isPresetFont,
} from "../../fonts";
import { EDITOR_OPTIONS, type DiffMode } from "../../types";

const FILES_PANE_MIN = 220;
const FILES_PANE_MAX = 800;
const FILES_PANE_DEFAULT = 320;

const {
  refreshIntervalSeconds,
  filesPaneWidth,
  setFilesPaneWidth,
  saveFilesPaneWidth,
  diffMode,
  saveDiffMode,
  diffFontFamily,
  saveDiffFontFamily,
  diffFontSize,
  setDiffFontSize,
  saveDiffFontSize,
  terminalFontFamily,
  saveTerminalFontFamily,
  terminalFontSize,
  setTerminalFontSize,
  saveTerminalFontSize,
  editor,
  saveEditor,
  saveRefreshInterval,
  showToast,
} = useApp();

const diffUsingCustom = ref(!isPresetFont(diffFontFamily.value));
const terminalUsingCustom = ref(!isPresetFont(terminalFontFamily.value));
const diffCustomFont = ref(diffUsingCustom.value ? diffFontFamily.value : "");
const terminalCustomFont = ref(terminalUsingCustom.value ? terminalFontFamily.value : "");

watch(diffFontFamily, (value) => {
  if (isPresetFont(value)) {
    diffUsingCustom.value = false;
    return;
  }
  diffUsingCustom.value = true;
  diffCustomFont.value = value;
});

watch(terminalFontFamily, (value) => {
  if (isPresetFont(value)) {
    terminalUsingCustom.value = false;
    return;
  }
  terminalUsingCustom.value = true;
  terminalCustomFont.value = value;
});

async function onRefreshInterval(event: Event) {
  const value = Number((event.target as HTMLSelectElement).value);
  try {
    await saveRefreshInterval(value);
  } catch (err) {
    showToast(String(err), "error");
  }
}

async function onDiffMode(mode: DiffMode) {
  try {
    await saveDiffMode(mode);
  } catch (err) {
    showToast(String(err), "error");
  }
}

async function onDiffFont(event: Event) {
  const value = (event.target as HTMLSelectElement).value;
  if (value === CUSTOM_FONT_ID) {
    diffUsingCustom.value = true;
    if (diffCustomFont.value.trim()) {
      try {
        await saveDiffFontFamily(diffCustomFont.value);
      } catch (err) {
        showToast(String(err), "error");
      }
    }
    return;
  }
  diffUsingCustom.value = false;
  try {
    await saveDiffFontFamily(value);
  } catch (err) {
    showToast(String(err), "error");
  }
}

async function onDiffCustomFont(event: Event) {
  const value = (event.target as HTMLInputElement).value;
  diffCustomFont.value = value;
  if (!value.trim()) {
    return;
  }
  try {
    await saveDiffFontFamily(value);
  } catch (err) {
    showToast(String(err), "error");
  }
}

function onDiffSizeInput(event: Event) {
  setDiffFontSize(Number((event.target as HTMLInputElement).value));
}

async function onDiffSizeChange(event: Event) {
  try {
    await saveDiffFontSize(Number((event.target as HTMLInputElement).value));
  } catch (err) {
    showToast(String(err), "error");
  }
}

async function resetDiffFontSize() {
  try {
    await saveDiffFontSize(DEFAULT_DIFF_FONT_SIZE);
  } catch (err) {
    showToast(String(err), "error");
  }
}

async function onTerminalFont(event: Event) {
  const value = (event.target as HTMLSelectElement).value;
  if (value === CUSTOM_FONT_ID) {
    terminalUsingCustom.value = true;
    if (terminalCustomFont.value.trim()) {
      try {
        await saveTerminalFontFamily(terminalCustomFont.value);
      } catch (err) {
        showToast(String(err), "error");
      }
    }
    return;
  }
  terminalUsingCustom.value = false;
  try {
    await saveTerminalFontFamily(value);
  } catch (err) {
    showToast(String(err), "error");
  }
}

async function onTerminalCustomFont(event: Event) {
  const value = (event.target as HTMLInputElement).value;
  terminalCustomFont.value = value;
  if (!value.trim()) {
    return;
  }
  try {
    await saveTerminalFontFamily(value);
  } catch (err) {
    showToast(String(err), "error");
  }
}

function onTerminalSizeInput(event: Event) {
  setTerminalFontSize(Number((event.target as HTMLInputElement).value));
}

async function onTerminalSizeChange(event: Event) {
  try {
    await saveTerminalFontSize(Number((event.target as HTMLInputElement).value));
  } catch (err) {
    showToast(String(err), "error");
  }
}

async function resetTerminalFontSize() {
  try {
    await saveTerminalFontSize(DEFAULT_TERMINAL_FONT_SIZE);
  } catch (err) {
    showToast(String(err), "error");
  }
}

async function onEditor(event: Event) {
  try {
    await saveEditor((event.target as HTMLSelectElement).value);
  } catch (err) {
    showToast(String(err), "error");
  }
}

function onWidthInput(event: Event) {
  setFilesPaneWidth(Number((event.target as HTMLInputElement).value));
}

async function onWidthChange(event: Event) {
  try {
    await saveFilesPaneWidth(Number((event.target as HTMLInputElement).value));
  } catch (err) {
    showToast(String(err), "error");
  }
}

async function resetFilesPaneWidth() {
  try {
    await saveFilesPaneWidth(FILES_PANE_DEFAULT);
  } catch (err) {
    showToast(String(err), "error");
  }
}
</script>

<template>
  <div class="settings-pane settings-form-page">
    <div class="settings-inner">
      <div class="settings-header">
        <div>
          <div class="brand">General</div>
          <p class="muted tiny">Preferences save as you change them.</p>
        </div>
      </div>

      <section class="settings-card">
        <div class="settings-row">
          <div class="settings-row-copy">
            <h3>Auto-fetch</h3>
            <p class="muted tiny">How often Shipyard fetches remote status for every repository.</p>
          </div>
          <label class="settings-control">
            <span class="visually-hidden">Auto-fetch interval</span>
            <select :value="refreshIntervalSeconds" @change="onRefreshInterval">
              <option :value="0">Off</option>
              <option :value="60">1 minute</option>
              <option :value="300">5 minutes</option>
              <option :value="900">15 minutes</option>
              <option :value="1800">30 minutes</option>
            </select>
          </label>
        </div>
        <div class="settings-row">
          <div class="settings-row-copy">
            <h3>Diff layout</h3>
            <p class="muted tiny">Default view when you open a file diff.</p>
          </div>
          <div class="segmented" role="group" aria-label="Diff layout">
            <button
              type="button"
              :class="{ active: diffMode === 'inline' }"
              :aria-pressed="diffMode === 'inline'"
              @click="onDiffMode('inline')"
            >
              Inline
            </button>
            <button
              type="button"
              :class="{ active: diffMode === 'split' }"
              :aria-pressed="diffMode === 'split'"
              @click="onDiffMode('split')"
            >
              Side by side
            </button>
          </div>
        </div>
        <div class="settings-row" :class="{ 'settings-row-stacked': diffUsingCustom }">
          <div class="settings-row-copy">
            <h3>Diff font</h3>
            <p class="muted tiny">Typeface used for file diffs in Changes and File History.</p>
          </div>
          <div class="settings-control settings-font">
            <label>
              <span class="visually-hidden">Diff font</span>
              <select :value="diffUsingCustom ? CUSTOM_FONT_ID : diffFontFamily" @change="onDiffFont">
                <option v-for="option in FONT_OPTIONS" :key="option.id" :value="option.id">
                  {{ option.label }}
                </option>
                <option :value="CUSTOM_FONT_ID">Custom…</option>
              </select>
            </label>
            <label v-if="diffUsingCustom">
              <span class="visually-hidden">Custom diff font</span>
              <input
                type="text"
                :value="diffCustomFont"
                placeholder="Font family name"
                autocapitalize="off"
                autocorrect="off"
                autocomplete="off"
                spellcheck="false"
                @change="onDiffCustomFont"
              />
            </label>
          </div>
        </div>
        <div class="settings-row">
          <div class="settings-row-copy">
            <h3>Diff size</h3>
            <p class="muted tiny">Size of the file diff text.</p>
          </div>
          <div class="settings-control settings-slider">
            <label class="settings-slider-input">
              <span class="visually-hidden">Diff font size</span>
              <input
                type="range"
                :min="FONT_SIZE_MIN"
                :max="FONT_SIZE_MAX"
                step="0.5"
                :value="diffFontSize"
                @input="onDiffSizeInput"
                @change="onDiffSizeChange"
              />
            </label>
            <span class="settings-slider-value">{{ formatFontSize(diffFontSize) }}</span>
            <button
              class="ghost tiny"
              type="button"
              :disabled="diffFontSize === DEFAULT_DIFF_FONT_SIZE"
              @click="resetDiffFontSize"
            >
              Reset
            </button>
          </div>
        </div>
        <div class="settings-row">
          <div class="settings-row-copy">
            <h3>Editor</h3>
            <p class="muted tiny">
              Used when you open a conflicted file. System default follows the macOS file
              association.
            </p>
          </div>
          <label class="settings-control">
            <span class="visually-hidden">Editor</span>
            <select :value="editor" @change="onEditor">
              <option
                v-for="option in EDITOR_OPTIONS"
                :key="option.id"
                :value="option.id"
              >
                {{ option.label }}
              </option>
              <option
                v-if="!EDITOR_OPTIONS.some((option) => option.id === editor)"
                :value="editor"
              >
                {{ editor }}
              </option>
            </select>
          </label>
        </div>
        <div class="settings-row">
          <div class="settings-row-copy">
            <h3>Files pane width</h3>
            <p class="muted tiny">Width of the working-tree list in a repository tab.</p>
          </div>
          <div class="settings-control settings-slider">
            <label class="settings-slider-input">
              <span class="visually-hidden">Files pane width</span>
              <input
                type="range"
                :min="FILES_PANE_MIN"
                :max="FILES_PANE_MAX"
                :value="filesPaneWidth"
                @input="onWidthInput"
                @change="onWidthChange"
              />
            </label>
            <span class="settings-slider-value">{{ filesPaneWidth }}px</span>
            <button
              class="ghost tiny"
              type="button"
              :disabled="filesPaneWidth === FILES_PANE_DEFAULT"
              @click="resetFilesPaneWidth"
            >
              Reset
            </button>
          </div>
        </div>
        <div class="settings-row" :class="{ 'settings-row-stacked': terminalUsingCustom }">
          <div class="settings-row-copy">
            <h3>Terminal font</h3>
            <p class="muted tiny">Typeface used in the repository terminal.</p>
          </div>
          <div class="settings-control settings-font">
            <label>
              <span class="visually-hidden">Terminal font</span>
              <select :value="terminalUsingCustom ? CUSTOM_FONT_ID : terminalFontFamily" @change="onTerminalFont">
                <option v-for="option in FONT_OPTIONS" :key="option.id" :value="option.id">
                  {{ option.label }}
                </option>
                <option :value="CUSTOM_FONT_ID">Custom…</option>
              </select>
            </label>
            <label v-if="terminalUsingCustom">
              <span class="visually-hidden">Custom terminal font</span>
              <input
                type="text"
                :value="terminalCustomFont"
                placeholder="Font family name"
                autocapitalize="off"
                autocorrect="off"
                autocomplete="off"
                spellcheck="false"
                @change="onTerminalCustomFont"
              />
            </label>
          </div>
        </div>
        <div class="settings-row">
          <div class="settings-row-copy">
            <h3>Terminal size</h3>
            <p class="muted tiny">Size of the repository terminal text.</p>
          </div>
          <div class="settings-control settings-slider">
            <label class="settings-slider-input">
              <span class="visually-hidden">Terminal font size</span>
              <input
                type="range"
                :min="FONT_SIZE_MIN"
                :max="FONT_SIZE_MAX"
                step="0.5"
                :value="terminalFontSize"
                @input="onTerminalSizeInput"
                @change="onTerminalSizeChange"
              />
            </label>
            <span class="settings-slider-value">{{ formatFontSize(terminalFontSize) }}</span>
            <button
              class="ghost tiny"
              type="button"
              :disabled="terminalFontSize === DEFAULT_TERMINAL_FONT_SIZE"
              @click="resetTerminalFontSize"
            >
              Reset
            </button>
          </div>
        </div>
      </section>
    </div>
  </div>
</template>
