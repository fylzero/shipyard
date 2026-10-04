<script setup lang="ts">
import { onMounted, onUnmounted } from "vue";
import { useBackdropClose } from "../composables/useBackdropClose";

defineProps<{
  title: string;
  wide?: boolean;
  medium?: boolean;
}>();

const emit = defineEmits<{
  close: [];
}>();

const backdrop = useBackdropClose(() => emit("close"));

function onKeydown(event: KeyboardEvent) {
  if (event.key === "Escape") {
    const target = event.target;
    if (target instanceof Element && target.closest("[data-modal-escape='local']")) {
      return;
    }
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
      class="modal-layer"
      @pointerdown="backdrop.onPointerdown"
      @click="backdrop.onClick"
    >
      <div class="modal" :class="{ wide, medium }" role="dialog" aria-modal="true">
        <div class="modal-title">{{ title }}</div>
        <div class="modal-body">
          <slot />
        </div>
        <div class="modal-actions">
          <slot name="actions" />
        </div>
      </div>
    </div>
  </Teleport>
</template>
