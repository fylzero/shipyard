<script setup lang="ts">
import { computed, onMounted, onUnmounted } from "vue";
import type { RepoActionResult } from "../types";
import { useBackdropClose } from "../composables/useBackdropClose";
import { baseName } from "../platform";

const props = defineProps<{
  title: string;
  results: RepoActionResult[];
}>();

const emit = defineEmits<{
  close: [];
}>();

const backdrop = useBackdropClose(() => emit("close"));

const failed = computed(() => props.results.filter((item) => !item.ok).length);

function isDetailed(message: string) {
  return message.includes("\n");
}

function onKeydown(event: KeyboardEvent) {
  if (event.key === "Escape") {
    event.stopImmediatePropagation();
    emit("close");
  }
}

onMounted(() => {
  document.addEventListener("keydown", onKeydown, true);
});

onUnmounted(() => {
  document.removeEventListener("keydown", onKeydown, true);
});
</script>

<template>
  <Teleport to="body">
    <div
      class="output-layer"
      @pointerdown="backdrop.onPointerdown"
      @click="backdrop.onClick"
    >
      <div class="output-modal" role="dialog" aria-modal="true" :aria-label="title">
        <div class="output-chrome">
          <div class="output-dots" aria-hidden="true">
            <span />
            <span />
            <span />
          </div>
          <div class="output-title">{{ title }}</div>
          <span v-if="failed" class="output-count">{{ failed }} failed</span>
        </div>
        <div class="output-body">
          <div
            v-for="result in results"
            :key="result.path"
            class="output-entry"
            :class="result.ok ? 'ok' : 'bad'"
          >
            <div class="output-line">
              <span class="output-name">{{ baseName(result.path) }}</span>
              <template v-if="!isDetailed(result.message)">
                <span class="output-sep"> — </span>
                <span class="output-message">{{ result.message }}</span>
              </template>
            </div>
            <pre v-if="isDetailed(result.message)" class="output-detail">{{ result.message }}</pre>
          </div>
        </div>
        <div class="output-actions">
          <button class="ghost" type="button" @click="emit('close')">Close</button>
        </div>
      </div>
    </div>
  </Teleport>
</template>
