<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref } from "vue";
import BranchIcon from "./BranchIcon.vue";
import { useBackdropClose } from "../composables/useBackdropClose";
import type { BranchConfirmOptions } from "../composables/useBranchConfirm";

const props = defineProps<BranchConfirmOptions>();

const emit = defineEmits<{
  answer: [ok: boolean];
}>();

const confirmButton = ref<HTMLButtonElement | null>(null);
const backdrop = useBackdropClose(() => emit("answer", false));

const total = computed(() => props.groups.reduce((sum, group) => sum + group.branches.length, 0));

function onKeydown(event: KeyboardEvent) {
  if (event.key === "Escape") {
    event.stopImmediatePropagation();
    emit("answer", false);
  }
}

onMounted(() => {
  document.addEventListener("keydown", onKeydown, true);
  void nextTick(() => confirmButton.value?.focus());
});

onUnmounted(() => {
  document.removeEventListener("keydown", onKeydown, true);
});
</script>

<template>
  <Teleport to="body">
    <div
      class="modal-layer"
      @pointerdown="backdrop.onPointerdown"
      @click="backdrop.onClick"
    >
      <div
        class="modal medium branch-confirm"
        role="alertdialog"
        aria-modal="true"
        :aria-label="title"
      >
        <div class="branch-confirm-head">
          <span class="branch-confirm-badge" aria-hidden="true">
            <svg viewBox="0 0 16 16" fill="currentColor">
              <path
                d="M11 1.75V3h2.25a.75.75 0 0 1 0 1.5H2.75a.75.75 0 0 1 0-1.5H5V1.75C5 .784 5.784 0 6.75 0h2.5C10.216 0 11 .784 11 1.75ZM4.496 6.675l.66 6.6a.25.25 0 0 0 .249.225h5.19a.25.25 0 0 0 .249-.225l.66-6.6a.75.75 0 0 1 1.492.149l-.66 6.6A1.748 1.748 0 0 1 10.595 15h-5.19a1.75 1.75 0 0 1-1.741-1.575l-.66-6.6a.75.75 0 1 1 1.492-.15ZM6.5 1.75V3h3V1.75a.25.25 0 0 0-.25-.25h-2.5a.25.25 0 0 0-.25.25Z"
              />
            </svg>
          </span>
          <div class="branch-confirm-heading">
            <div class="modal-title branch-confirm-title">{{ title }}</div>
            <p class="branch-confirm-message">{{ message }}</p>
          </div>
        </div>

        <div class="branch-confirm-list">
          <section v-for="group in groups" :key="group.name" class="branch-confirm-group">
            <header v-if="group.name" class="branch-confirm-repo">
              <span class="branch-confirm-repo-name">{{ group.name }}</span>
              <span class="branch-confirm-count">{{ group.branches.length }}</span>
            </header>
            <ul class="branch-confirm-branches">
              <li v-for="branch in group.branches" :key="branch" class="branch-confirm-chip">
                <BranchIcon />
                <span>{{ branch }}</span>
              </li>
            </ul>
          </section>
        </div>

        <p v-if="note" class="branch-confirm-note">{{ note }}</p>

        <div class="modal-actions">
          <button class="ghost" type="button" @click="emit('answer', false)">
            {{ cancelLabel ?? "Cancel" }}
          </button>
          <button
            ref="confirmButton"
            class="destructive"
            type="button"
            @click="emit('answer', true)"
          >
            {{ confirmLabel }}{{ total > 1 ? ` ${total}` : "" }}
          </button>
        </div>
      </div>
    </div>
  </Teleport>
</template>
