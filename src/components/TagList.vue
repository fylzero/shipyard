<script setup lang="ts">
import { ref } from "vue";
import BranchContextMenu from "./BranchContextMenu.vue";
import type { TagEntry } from "../types";
import { formatCommitDate } from "../graphLayout";

defineProps<{
  tags: TagEntry[];
  busy: boolean;
}>();

const emit = defineEmits<{
  create: [];
  delete: [tag: TagEntry];
}>();

const nameMenu = ref<{ name: string; x: number; y: number } | null>(null);

function keepNameUnselected(event: MouseEvent) {
  if (event.button === 2) {
    event.preventDefault();
  }
}

function openNameMenu(event: MouseEvent, name: string) {
  event.preventDefault();
  event.stopPropagation();
  window.getSelection()?.removeAllRanges();
  nameMenu.value = { name, x: event.clientX, y: event.clientY };
}
</script>

<template>
  <div class="branch-pane">
    <div class="remote-header">
      <p class="muted tiny tag-header-copy">
        Tags mark a commit. Annotated tags keep a message; lightweight tags are just a name.
      </p>
      <div class="remote-header-actions">
        <button class="ghost tiny" type="button" :disabled="busy" @click="emit('create')">
          <svg class="button-icon" viewBox="0 0 24 24" aria-hidden="true">
            <path
              d="M9.568 3H5.25A2.25 2.25 0 003 5.25v4.318c0 .597.237 1.17.659 1.591l9.581 9.581c.699.699 1.78.872 2.607.33a18.095 18.095 0 005.223-5.223c.542-.827.369-1.908-.33-2.607L11.16 3.66A2.25 2.25 0 009.568 3Z"
            />
            <path d="M6 6h.01" />
          </svg>
          New tag
        </button>
      </div>
    </div>
    <div class="graph-scroll branch-list">
      <p v-if="!tags.length" class="muted tiny empty-files">No tags.</p>
      <div
        v-for="tag in tags"
        :key="tag.name"
        class="branch-row tag-row"
        @mousedown="keepNameUnselected"
        @contextmenu="openNameMenu($event, tag.name)"
      >
        <svg class="button-icon tag-row-icon" viewBox="0 0 24 24" aria-hidden="true">
          <path
            d="M9.568 3H5.25A2.25 2.25 0 003 5.25v4.318c0 .597.237 1.17.659 1.591l9.581 9.581c.699.699 1.78.872 2.607.33a18.095 18.095 0 005.223-5.223c.542-.827.369-1.908-.33-2.607L11.16 3.66A2.25 2.25 0 009.568 3Z"
          />
          <path d="M6 6h.01" />
        </svg>
        <span class="branch-row-name" :title="tag.name">{{ tag.name }}</span>
        <span v-if="tag.annotated" class="branch-pill annotated">Annotated</span>
        <span v-if="tag.message" class="tag-row-message" :title="tag.message">{{ tag.message }}</span>
        <span v-if="tag.hash" class="tag-row-hash" :title="tag.hash">{{ tag.hash }}</span>
        <span v-if="tag.date" class="stash-row-date">{{ formatCommitDate(tag.date) }}</span>
        <div class="stash-row-actions">
          <button
            class="ghost tiny danger"
            type="button"
            :disabled="busy"
            :title="`Delete ${tag.name}`"
            @click="emit('delete', tag)"
          >
            <svg class="button-icon" viewBox="0 0 24 24" aria-hidden="true">
              <path d="M6 18 18 6M6 6l12 12" />
            </svg>
            Delete
          </button>
        </div>
      </div>
    </div>
    <BranchContextMenu
      v-if="nameMenu"
      :branch="nameMenu.name"
      :x="nameMenu.x"
      :y="nameMenu.y"
      copy-label="Copy tag name"
      copied-message="Copied tag name"
      @close="nameMenu = null"
    />
  </div>
</template>
