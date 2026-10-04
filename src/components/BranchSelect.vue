<script setup lang="ts">
import { nextTick, onUnmounted, ref } from "vue";
import BranchPicker from "./BranchPicker.vue";
import type { BranchTracking } from "../types";

const props = defineProps<{
  modelValue: string;
  branches: string[];
  branchTracking?: BranchTracking[];
  current?: string;
  label: string;
  emptyText?: string;
}>();

const emit = defineEmits<{
  "update:modelValue": [branch: string];
}>();

const DROPDOWN_MAX_HEIGHT = 288;
const DROPDOWN_MIN_HEIGHT = 180;
const VIEWPORT_MARGIN = 12;

const rootEl = ref<HTMLElement | null>(null);
const triggerEl = ref<HTMLButtonElement | null>(null);
const isOpen = ref(false);
const openAbove = ref(false);
const maxHeight = ref(DROPDOWN_MAX_HEIGHT);
const initialQuery = ref("");

function onDocumentPointerDown(event: PointerEvent) {
  const target = event.target;
  if (target instanceof Node && rootEl.value?.contains(target)) {
    return;
  }
  close(false);
}

function open(query = "") {
  if (isOpen.value || !props.branches.length) {
    return;
  }
  const rect = triggerEl.value?.getBoundingClientRect();
  if (rect) {
    const below = window.innerHeight - rect.bottom - VIEWPORT_MARGIN;
    const above = rect.top - VIEWPORT_MARGIN;
    openAbove.value = below < DROPDOWN_MIN_HEIGHT && above > below;
    maxHeight.value = Math.min(DROPDOWN_MAX_HEIGHT, openAbove.value ? above : below);
  }
  initialQuery.value = query;
  isOpen.value = true;
  document.addEventListener("pointerdown", onDocumentPointerDown, true);
}

function close(refocus = true) {
  if (!isOpen.value) {
    return;
  }
  isOpen.value = false;
  document.removeEventListener("pointerdown", onDocumentPointerDown, true);
  if (refocus) {
    void nextTick(() => triggerEl.value?.focus());
  }
}

function toggle() {
  if (isOpen.value) {
    close();
  } else {
    open();
  }
}

function select(branch: string) {
  close();
  if (branch !== props.modelValue) {
    emit("update:modelValue", branch);
  }
}

function onTriggerKeydown(event: KeyboardEvent) {
  if (event.key === "ArrowDown" || event.key === "ArrowUp") {
    event.preventDefault();
    open();
    return;
  }
  if (event.key.length === 1 && event.key !== " " && !event.metaKey && !event.ctrlKey && !event.altKey) {
    event.preventDefault();
    open(event.key);
  }
}

function onDropdownKeydown(event: KeyboardEvent) {
  if (event.key === "Escape" && !event.defaultPrevented) {
    event.stopPropagation();
    event.preventDefault();
    close();
    return;
  }
  if (event.key === "Tab") {
    close(false);
  }
}

onUnmounted(() => {
  document.removeEventListener("pointerdown", onDocumentPointerDown, true);
});
</script>

<template>
  <div ref="rootEl" class="branch-select">
    <button
      ref="triggerEl"
      class="branch-select-trigger"
      type="button"
      :disabled="!branches.length"
      :aria-expanded="isOpen"
      aria-haspopup="menu"
      :aria-label="label"
      :title="modelValue || undefined"
      @click="toggle"
      @keydown="onTriggerKeydown"
    >
      <span class="branch-select-value" :class="{ muted: !modelValue }">
        {{ modelValue || (branches.length ? "Choose a branch" : emptyText ?? "No local branches") }}
      </span>
      <span v-if="current && modelValue === current" class="branch-pill">Current</span>
      <svg viewBox="0 0 16 16" aria-hidden="true">
        <path d="M4.2 6.2L8 10l3.8-3.8" />
      </svg>
    </button>
    <div
      v-if="isOpen"
      class="overflow-menu-dropdown branch-select-dropdown"
      :class="{ above: openAbove }"
      :style="{ maxHeight: `${maxHeight}px` }"
      role="menu"
      :aria-label="label"
      data-modal-escape="local"
      @keydown="onDropdownKeydown"
    >
      <BranchPicker
        :branches="branches"
        :branch-tracking="branchTracking"
        :selected="modelValue"
        :current="current"
        :initial-query="initialQuery"
        :empty-text="emptyText"
        @select="select"
        @close="close()"
      />
    </div>
  </div>
</template>
